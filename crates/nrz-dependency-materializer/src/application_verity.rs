//! Bounded package byte construction/readback; these results grant no custody or publication authority.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

use nrz_contract::application_package_v1::{VerityGeometry, VerityHashAlgorithm};
use nrz_runtime_artifact::application_package_v1::{
    APPLICATION_VERITY_BLOCK_SIZE, APPLICATION_VERITY_HASHES_PER_BLOCK, ApplicationPackageLayout,
    application_package_layout,
};
use sha2::{Digest, Sha256};

const BLOCK_BYTES: usize = APPLICATION_VERITY_BLOCK_SIZE as usize;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ApplicationVerityError {
    #[error("application package I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid application package: {0}")]
    Invalid(String),
}

#[derive(Debug)]
pub(crate) struct ComputedApplicationPackage {
    pub(crate) verity: VerityGeometry,
    pub(crate) blob_digest: String,
    pub(crate) blob_size: u64,
}

#[derive(Debug)]
struct Layer {
    blocks: u64,
    offset: u64,
}

/// The owner must supply a new private image inode and retain exclusive custody.
pub(crate) fn append_application_verity(
    file: &mut File,
    erofs_size: u64,
    salt: [u8; 32],
) -> Result<ComputedApplicationPackage, ApplicationVerityError> {
    require(
        file.metadata()?.is_file(),
        "package inode must be a regular file",
    )?;
    let ApplicationPackageLayout {
        data_block_count: data_blocks,
        tree_offset,
        tree_length,
        package_size,
    } = application_package_layout(erofs_size)
        .map_err(|error| ApplicationVerityError::Invalid(error.to_string()))?;
    require(
        file.seek(SeekFrom::End(0))? == erofs_size,
        "image length changed",
    )?;
    let padding = tree_offset - erofs_size;
    file.write_all(&[0_u8; BLOCK_BYTES][..padding as usize])?;
    file.set_len(package_size)?;
    let layers = layers(data_blocks, tree_offset)?;
    for index in 0..layers.len() {
        visit_layer(file, &layers, index, data_blocks, &salt, false)?;
    }
    let root_hash = hex::encode(read_root(file, &layers, &salt)?);
    let verity = VerityGeometry {
        version: 1,
        hash_algorithm: VerityHashAlgorithm::Sha256,
        root_hash,
        salt: hex::encode(salt),
        data_block_size: BLOCK_BYTES as u32,
        hash_block_size: BLOCK_BYTES as u32,
        data_block_count: data_blocks,
        data_offset: 0,
        tree_offset,
        tree_length,
    };
    // Independent readback also catches failed writes before the owner creates a descriptor.
    verify_application_verity(file, erofs_size, &verity)
}

pub(crate) fn verify_application_verity(
    file: &mut File,
    erofs_size: u64,
    verity: &VerityGeometry,
) -> Result<ComputedApplicationPackage, ApplicationVerityError> {
    require(
        file.metadata()?.is_file(),
        "package inode must be a regular file",
    )?;
    let ApplicationPackageLayout {
        data_block_count: data_blocks,
        tree_offset,
        tree_length,
        package_size,
    } = application_package_layout(erofs_size)
        .map_err(|error| ApplicationVerityError::Invalid(error.to_string()))?;
    require(
        verity.version == 1
            && verity.hash_algorithm == VerityHashAlgorithm::Sha256
            && verity.data_block_size == BLOCK_BYTES as u32
            && verity.hash_block_size == BLOCK_BYTES as u32
            && verity.data_block_count == data_blocks
            && verity.data_offset == 0
            && verity.tree_offset == tree_offset
            && verity.tree_length == tree_length,
        "unsupported or inconsistent verity geometry",
    )?;
    let salt = decode_hash(&verity.salt)?;
    let expected_root = decode_hash(&verity.root_hash)?;
    require(
        file.seek(SeekFrom::End(0))? == package_size,
        "package length mismatch",
    )?;
    let mut padding = [0_u8; BLOCK_BYTES];
    let padding = &mut padding[..(tree_offset - erofs_size) as usize];
    file.seek(SeekFrom::Start(erofs_size))?;
    file.read_exact(padding)?;
    require(
        padding.iter().all(|byte| *byte == 0),
        "nonzero data padding",
    )?;
    let layers = layers(data_blocks, tree_offset)?;
    for index in 0..layers.len() {
        visit_layer(file, &layers, index, data_blocks, &salt, true)?;
    }
    require(
        read_root(file, &layers, &salt)? == expected_root,
        "verity root mismatch",
    )?;
    file.seek(SeekFrom::Start(0))?;
    let mut digest = blake3::Hasher::new();
    let mut buffer = [0_u8; 64 * 1024];
    let mut remaining = package_size;
    while remaining > 0 {
        let length = remaining.min(buffer.len() as u64) as usize;
        file.read_exact(&mut buffer[..length])?;
        digest.update(&buffer[..length]);
        remaining -= length as u64;
    }
    require(
        file.seek(SeekFrom::End(0))? == package_size,
        "package length changed",
    )?;
    Ok(ComputedApplicationPackage {
        verity: verity.clone(),
        blob_digest: digest.finalize().to_hex().to_string(),
        blob_size: package_size,
    })
}

fn layers(data_blocks: u64, tree_offset: u64) -> Result<Vec<Layer>, ApplicationVerityError> {
    let mut child_blocks = data_blocks;
    let mut layers = Vec::new();
    while child_blocks > 1 {
        child_blocks = child_blocks.div_ceil(APPLICATION_VERITY_HASHES_PER_BLOCK);
        layers.push(Layer {
            blocks: child_blocks,
            offset: 0,
        });
    }
    // dm-verity stores the root level first, while construction runs bottom-up.
    let mut offset = tree_offset;
    for layer in layers.iter_mut().rev() {
        layer.offset = offset;
        offset = offset
            .checked_add(
                layer
                    .blocks
                    .checked_mul(APPLICATION_VERITY_BLOCK_SIZE)
                    .ok_or_else(|| invalid("tree length overflow"))?,
            )
            .ok_or_else(|| invalid("tree offset overflow"))?;
    }
    Ok(layers)
}

fn visit_layer(
    file: &mut File,
    layers: &[Layer],
    index: usize,
    data_blocks: u64,
    salt: &[u8; 32],
    verify: bool,
) -> Result<(), ApplicationVerityError> {
    let (child_blocks, child_offset) = if index == 0 {
        (data_blocks, 0)
    } else {
        (layers[index - 1].blocks, layers[index - 1].offset)
    };
    let mut child = [0_u8; BLOCK_BYTES];
    let mut parent = [0_u8; BLOCK_BYTES];
    let mut child_index = 0_u64;
    for parent_index in 0..layers[index].blocks {
        parent.fill(0);
        for slot in parent.as_chunks_mut::<32>().0 {
            if child_index == child_blocks {
                break;
            }
            read_block(file, block_offset(child_offset, child_index)?, &mut child)?;
            slot.copy_from_slice(&hash_block(salt, &child));
            child_index += 1;
        }
        let offset = block_offset(layers[index].offset, parent_index)?;
        file.seek(SeekFrom::Start(offset))?;
        if verify {
            file.read_exact(&mut child)?;
            require(child == parent, "verity tree hash or padding mismatch")?;
        } else {
            file.write_all(&parent)?;
        }
    }
    Ok(())
}

fn read_root(
    file: &mut File,
    layers: &[Layer],
    salt: &[u8; 32],
) -> Result<[u8; 32], ApplicationVerityError> {
    let mut block = [0_u8; BLOCK_BYTES];
    read_block(
        file,
        layers.last().map_or(0, |layer| layer.offset),
        &mut block,
    )?;
    Ok(hash_block(salt, &block))
}

fn hash_block(salt: &[u8; 32], block: &[u8; BLOCK_BYTES]) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(salt);
    digest.update(block);
    digest.finalize().into()
}

fn read_block(
    file: &mut File,
    offset: u64,
    block: &mut [u8; BLOCK_BYTES],
) -> Result<(), ApplicationVerityError> {
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(block)?;
    Ok(())
}

fn block_offset(start: u64, index: u64) -> Result<u64, ApplicationVerityError> {
    index
        .checked_mul(APPLICATION_VERITY_BLOCK_SIZE)
        .and_then(|value| start.checked_add(value))
        .ok_or_else(|| invalid("block offset overflow"))
}

fn decode_hash(value: &str) -> Result<[u8; 32], ApplicationVerityError> {
    require(
        value.len() == 64
            && value
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)),
        "hash must be 32-byte lowercase hexadecimal",
    )?;
    let mut hash = [0; 32];
    hex::decode_to_slice(value, &mut hash).map_err(|_| invalid("invalid hexadecimal hash"))?;
    Ok(hash)
}

fn invalid(message: &str) -> ApplicationVerityError {
    ApplicationVerityError::Invalid(message.into())
}

fn require(condition: bool, message: &str) -> Result<(), ApplicationVerityError> {
    if condition {
        Ok(())
    } else {
        Err(invalid(message))
    }
}

#[cfg(test)]
#[path = "application_verity_tests.rs"]
mod tests;
