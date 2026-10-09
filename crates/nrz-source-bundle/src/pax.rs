pub fn encode_pax_record(fields: &[(&str, &str)]) -> String {
    fields
        .iter()
        .map(|(key, value)| encode_pax_key_value_record(key, value))
        .collect()
}

fn encode_pax_key_value_record(key: &str, value: &str) -> String {
    let body = format!("{key}={value}\n");
    let mut length = format!("0 {body}").len();
    loop {
        let record = format!("{length} {body}");
        let byte_length = record.len();
        if byte_length == length {
            return record;
        }
        length = byte_length;
    }
}
