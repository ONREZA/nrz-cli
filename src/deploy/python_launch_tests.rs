use std::process::Command;

use super::python_launch::materialize_python_entry;
use crate::detect::fs::LocalFs;
use crate::detect::python_launch::{
    PythonLaunch, PythonLaunchRequest, resolve_launch_for_framework,
};

fn resolve_launch(
    fs: &dyn crate::detect::fs::Fs,
    request: PythonLaunchRequest<'_>,
) -> anyhow::Result<Option<PythonLaunch>> {
    resolve_launch_for_framework(fs, request, None)
}

#[test]
fn python_module_bootstrap_preserves_literal_arguments_and_src_imports() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join("src/demo")).unwrap();
    std::fs::write(project.path().join("src/demo/__init__.py"), "").unwrap();
    std::fs::write(
        project.path().join("src/demo/__main__.py"),
        "import json, sys\nprint(json.dumps({'args':sys.argv[1:], 'name':__name__}))\n",
    )
    .unwrap();
    let arguments = vec![
        "spaces and quotes ' \"".into(),
        "$(touch should-not-exist)".into(),
        "line\nbreak".into(),
    ];
    let launch = resolve_launch(
        &LocalFs::new(project.path()),
        PythonLaunchRequest {
            entry: None,
            module: Some("demo"),
            application: None,
            server: None,
            args: &arguments,
        },
    )
    .unwrap()
    .unwrap();
    let entry = materialize_python_entry(
        project.path(),
        &launch,
        nrz_source_bundle::PythonMinor::default(),
    )
    .unwrap();
    let result = Command::new("python3")
        .arg(project.path().join(&entry))
        .args(&launch.args)
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let observed: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(observed["args"], serde_json::json!(arguments));
    assert_eq!(observed["name"], "__main__");
    assert!(!project.path().join("should-not-exist").exists());
}

#[test]
fn python_bootstrap_bytes_are_independent_of_frozen_launch_intent() {
    let project = tempfile::tempdir().unwrap();
    let mut first_bootstrap = None;
    for module in ["first", "second"] {
        let launch = resolve_launch(
            &LocalFs::new(project.path()),
            PythonLaunchRequest {
                entry: None,
                module: Some(module),
                application: None,
                server: None,
                args: &[],
            },
        )
        .unwrap()
        .unwrap();
        let entry = materialize_python_entry(
            project.path(),
            &launch,
            nrz_source_bundle::PythonMinor::default(),
        )
        .unwrap();
        let bootstrap = std::fs::read(project.path().join(entry)).unwrap();
        if let Some(first) = &first_bootstrap {
            assert_eq!(&bootstrap, first);
        } else {
            first_bootstrap = Some(bootstrap);
        }
        assert_eq!(launch.args, ["MODULE", module]);
    }
}

#[test]
fn python_module_bootstrap_prefers_installed_application_package_over_source_layout() {
    let project = tempfile::tempdir().unwrap();
    for (base, value) in [
        ("src/demo", "source"),
        (".onreza/python/3.14/site-packages/demo", "installed"),
    ] {
        std::fs::create_dir_all(project.path().join(base)).unwrap();
        std::fs::write(project.path().join(base).join("__init__.py"), "").unwrap();
        std::fs::write(
            project.path().join(base).join("__main__.py"),
            format!("print('{value}')\n"),
        )
        .unwrap();
    }
    let launch = resolve_launch(
        &LocalFs::new(project.path()),
        PythonLaunchRequest {
            entry: None,
            module: Some("demo"),
            application: None,
            server: None,
            args: &[],
        },
    )
    .unwrap()
    .unwrap();
    let entry = materialize_python_entry(
        project.path(),
        &launch,
        nrz_source_bundle::PythonMinor::default(),
    )
    .unwrap();
    let result = Command::new("python3")
        .arg(project.path().join(&entry))
        .args(&launch.args)
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(String::from_utf8(result.stdout).unwrap(), "installed\n");
}

#[test]
fn python_django_launch_infers_wsgi_and_supports_explicit_asgi() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("manage.py"), "").unwrap();
    std::fs::create_dir_all(project.path().join("site")).unwrap();
    for file in ["wsgi.py", "asgi.py"] {
        std::fs::write(project.path().join("site").join(file), "").unwrap();
    }
    for (server, expected) in [
        (None, ["WSGI", "site.wsgi:application"]),
        (Some("asgi"), ["ASGI", "site.asgi:application"]),
    ] {
        let launch = resolve_launch(
            &LocalFs::new(project.path()),
            PythonLaunchRequest {
                entry: None,
                module: None,
                application: None,
                server,
                args: &[],
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(launch.args, expected);
    }
}

#[test]
fn python_server_missing_production_dependency_has_actionable_error() {
    let project = tempfile::tempdir().unwrap();
    let launch = resolve_launch(
        &LocalFs::new(project.path()),
        PythonLaunchRequest {
            entry: None,
            module: None,
            application: Some("main:app"),
            server: Some("asgi"),
            args: &[],
        },
    )
    .unwrap()
    .unwrap();
    let error = materialize_python_entry(
        project.path(),
        &launch,
        nrz_source_bundle::PythonMinor::default(),
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("uvicorn in installed production dependencies")
    );
    assert!(error.to_string().contains("poetry add uvicorn"));
}

#[test]
fn python_ambiguous_or_conflicting_launch_is_rejected() {
    let project = tempfile::tempdir().unwrap();
    for file in ["main.py", "app.py"] {
        std::fs::write(project.path().join(file), "").unwrap();
    }
    let fs = LocalFs::new(project.path());
    assert!(
        resolve_launch(
            &fs,
            PythonLaunchRequest {
                entry: Some(crate::detect::python_launch::PYTHON_BOOTSTRAP_ENTRY),
                module: None,
                application: None,
                server: None,
                args: &[]
            }
        )
        .unwrap_err()
        .to_string()
        .contains("reserved")
    );
    assert!(
        resolve_launch(
            &fs,
            PythonLaunchRequest {
                entry: None,
                module: None,
                application: None,
                server: None,
                args: &[]
            }
        )
        .unwrap_err()
        .to_string()
        .contains("multiple Python entry")
    );
    assert!(
        resolve_launch(
            &fs,
            PythonLaunchRequest {
                entry: Some("main.py"),
                module: Some("app"),
                application: None,
                server: None,
                args: &[]
            }
        )
        .unwrap_err()
        .to_string()
        .contains("mutually exclusive")
    );
    assert!(
        resolve_launch(
            &fs,
            PythonLaunchRequest {
                entry: None,
                module: Some("../bad"),
                application: None,
                server: None,
                args: &[]
            }
        )
        .is_err()
    );
}

#[test]
fn python_script_preserves_all_64_arguments_and_module_reserves_intent_slots() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(project.path().join("main.py"), "").unwrap();
    let args: Vec<_> = (0..64).map(|number| number.to_string()).collect();
    let script = resolve_launch(
        &LocalFs::new(project.path()),
        PythonLaunchRequest {
            entry: None,
            module: None,
            application: None,
            server: None,
            args: &args,
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(script.args, args);
    assert!(
        resolve_launch(
            &LocalFs::new(project.path()),
            PythonLaunchRequest {
                entry: None,
                module: Some("demo"),
                application: None,
                server: None,
                args: &args
            }
        )
        .unwrap_err()
        .to_string()
        .contains("62 user arguments")
    );
}

#[test]
fn python_declared_console_script_freezes_callable_and_runs_literal_arguments() {
    for metadata in [
        "[project]\nname='demo'\ndependencies=['fastapi','uvicorn']\n[project.scripts]\ndemo='demo:main'\n",
        "[tool.poetry]\nname='demo'\n[tool.poetry.dependencies]\nfastapi='*'\nuvicorn='*'\n[tool.poetry.scripts]\ndemo={reference='demo:main',type='console'}\n",
    ] {
        let project = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(project.path().join("src/demo")).unwrap();
        std::fs::write(project.path().join("pyproject.toml"), metadata).unwrap();
        if metadata.contains("[tool.poetry]") {
            std::fs::write(project.path().join("poetry.lock"), "").unwrap();
        }
        std::fs::write(
            project.path().join("src/demo/__init__.py"),
            "import sys,json\ndef main():\n print(json.dumps(sys.argv[1:]))\n return 0\n",
        )
        .unwrap();
        let args = vec!["literal $(touch forbidden) ' \"".to_string()];
        let launch = resolve_launch(
            &LocalFs::new(project.path()),
            PythonLaunchRequest {
                entry: None,
                module: None,
                application: None,
                server: None,
                args: &args,
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(launch.args[..2], ["CALLABLE", "demo:main"]);
        let entry = materialize_python_entry(
            project.path(),
            &launch,
            nrz_source_bundle::PythonMinor::default(),
        )
        .unwrap();
        let result = Command::new("python3")
            .arg(project.path().join(&entry))
            .args(&launch.args)
            .current_dir(project.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert_eq!(
            serde_json::from_slice::<Vec<String>>(&result.stdout).unwrap(),
            args
        );
        assert!(!project.path().join("forbidden").exists());
    }
}

#[test]
fn python_multiple_console_scripts_require_explicit_selection() {
    let project = tempfile::tempdir().unwrap();
    std::fs::write(
        project.path().join("pyproject.toml"),
        "[project]\nname='demo'\n[project.scripts]\nfirst='demo:first'\nsecond='demo:second'\n",
    )
    .unwrap();
    assert!(
        resolve_launch(
            &LocalFs::new(project.path()),
            PythonLaunchRequest {
                entry: None,
                module: None,
                application: None,
                server: None,
                args: &[]
            }
        )
        .unwrap_err()
        .to_string()
        .contains("multiple declared Python console scripts")
    );
    let launch = resolve_launch(
        &LocalFs::new(project.path()),
        PythonLaunchRequest {
            entry: None,
            module: None,
            application: Some("demo:second"),
            server: None,
            args: &[],
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(launch.args, ["CALLABLE", "demo:second"]);
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "downloads authored framework/server dependencies and executes real HTTP servers"]
async fn python_framework_server_http_port_and_graceful_shutdown_qualification() {
    use super::python_toolchain::{PythonInstallMode, install_commands};
    let mode = super::python_toolchain_tests::qualification_mode();
    let uv = super::python_toolchain::resolve_for(mode).await.unwrap();
    for minor in nrz_source_bundle::PythonMinor::ALL {
        let exact = minor.exact_version();
        if mode == PythonInstallMode::ManagedLocal {
            let installed = Command::new(&uv)
                .args(["python", "install", exact])
                .output()
                .unwrap();
            assert!(
                installed.status.success(),
                "{}",
                String::from_utf8_lossy(&installed.stderr)
            );
        }
        let interpreter = super::python_toolchain_tests::qualification_interpreter(&uv, minor);
        for (framework, requirements, source, server, application) in [
            (
                "fastapi",
                "fastapi==0.142.2\nuvicorn==0.54.0\n",
                "from fastapi import FastAPI\napp=FastAPI()\n@app.get('/')\ndef index(): return {'framework':'fastapi'}\n",
                None,
                None,
            ),
            (
                "starlette",
                "starlette==1.7.0\nuvicorn==0.54.0\n",
                "from starlette.applications import Starlette\nfrom starlette.responses import JSONResponse\nfrom starlette.routing import Route\nasync def index(request): return JSONResponse({'framework':'starlette'})\napp=Starlette(routes=[Route('/',index)])\n",
                None,
                None,
            ),
            (
                "flask",
                "flask==3.1.3\ngunicorn==26.2.0\n",
                "from flask import Flask\napp=Flask(__name__)\n@app.get('/')\ndef index(): return {'framework':'flask'}\n",
                None,
                None,
            ),
            (
                "django",
                "django==6.1.2\ngunicorn==26.2.0\n",
                "",
                None,
                None,
            ),
            (
                "django",
                "django==6.1.2\nuvicorn==0.54.0\n",
                "",
                Some("asgi"),
                None,
            ),
            (
                "generic-asgi",
                "uvicorn==0.54.0\n",
                "async def app(scope,receive,send):\n if scope['type']!='http': raise ValueError('no lifespan')\n await send({'type':'http.response.start','status':200,'headers':[(b'content-type',b'application/json')]})\n await send({'type':'http.response.body','body':b'{\"framework\":\"generic-asgi\"}'})\ndef create_app(): return app\n",
                Some("asgi"),
                Some("main:create_app()"),
            ),
            (
                "generic-wsgi",
                "gunicorn==26.2.0\n",
                "def app(environ,start_response):\n start_response('200 OK',[('Content-Type','application/json')])\n return [b'{\"framework\":\"generic-wsgi\"}']\ndef create_app(): return app\n",
                Some("wsgi"),
                Some("main:create_app()"),
            ),
        ] {
            let project = tempfile::tempdir().unwrap();
            std::fs::write(project.path().join("requirements.txt"), requirements).unwrap();
            if framework == "django" {
                std::fs::write(project.path().join("manage.py"), "").unwrap();
                std::fs::create_dir_all(project.path().join("config")).unwrap();
                for (file, contents) in [
                    ("__init__.py", ""),
                    (
                        "settings.py",
                        "SECRET_KEY='test'\nALLOWED_HOSTS=['*']\nROOT_URLCONF='config.urls'\nINSTALLED_APPS=[]\n",
                    ),
                    (
                        "urls.py",
                        "from django.http import JsonResponse\nfrom django.urls import path\ndef index(request): return JsonResponse({'framework':'django'})\nurlpatterns=[path('',index)]\n",
                    ),
                    (
                        "wsgi.py",
                        "import os\nos.environ.setdefault('DJANGO_SETTINGS_MODULE','config.settings')\nfrom django.core.wsgi import get_wsgi_application\napplication=get_wsgi_application()\n",
                    ),
                    (
                        "asgi.py",
                        "import os\nos.environ.setdefault('DJANGO_SETTINGS_MODULE','config.settings')\nfrom django.core.asgi import get_asgi_application\napplication=get_asgi_application()\n",
                    ),
                ] {
                    std::fs::write(project.path().join("config").join(file), contents).unwrap();
                }
            } else {
                std::fs::write(project.path().join("main.py"), source).unwrap();
            }
            for command in install_commands(project.path(), mode, "linux", "x86_64", minor).unwrap()
            {
                let program = if command.program.as_os_str().is_empty() {
                    uv.as_path()
                } else {
                    command.program.as_path()
                };
                let result = Command::new(program)
                    .args(&command.arguments)
                    .current_dir(project.path())
                    .output()
                    .unwrap();
                assert!(
                    result.status.success(),
                    "{framework}: {}",
                    String::from_utf8_lossy(&result.stderr)
                );
            }
            let launch = resolve_launch(
                &LocalFs::new(project.path()),
                PythonLaunchRequest {
                    entry: None,
                    module: None,
                    application,
                    server,
                    args: &[],
                },
            )
            .unwrap()
            .unwrap();
            let entry = materialize_python_entry(project.path(), &launch, minor).unwrap();
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            drop(listener);
            let log = tempfile::NamedTempFile::new().unwrap();
            let child = Command::new(&interpreter)
                .arg(project.path().join(&entry))
                .args(&launch.args)
                .current_dir(project.path())
                .env(
                    "PYTHONPATH",
                    project.path().join(minor.site_packages_root()),
                )
                .env("PORT", port.to_string())
                .stdout(std::process::Stdio::null())
                .stderr(log.reopen().unwrap())
                .spawn()
                .unwrap();
            super::python_toolchain_tests::qualification_artifact(
                project.path(),
                minor,
                if framework == "django" && server == Some("asgi") {
                    "django-asgi"
                } else {
                    framework
                },
                &entry,
                &launch.args,
                Some(serde_json::json!({"framework":framework})),
            )
            .await;
            let mut child = PythonServer(child);
            let mut observed = None;
            for _ in 0..100 {
                if let Ok(response) = reqwest::get(format!("http://127.0.0.1:{port}/")).await
                    && response.status().is_success()
                {
                    observed = Some(response.json::<serde_json::Value>().await.unwrap());
                    break;
                }
                if child.0.try_wait().unwrap().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            assert_eq!(
                observed,
                Some(serde_json::json!({"framework":framework})),
                "{}",
                std::fs::read_to_string(log.path()).unwrap()
            );
            assert!(
                Command::new("kill")
                    .args(["-TERM", &child.0.id().to_string()])
                    .status()
                    .unwrap()
                    .success()
            );
            let mut status = None;
            for _ in 0..100 {
                status = child.0.try_wait().unwrap();
                if status.is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            use std::os::unix::process::ExitStatusExt;
            let shutdown_log = std::fs::read_to_string(log.path()).unwrap();
            assert!(
                status.is_some_and(|status| status.success() || status.signal() == Some(15))
                    && (shutdown_log.contains("Finished server process")
                        || shutdown_log.contains("Shutting down: Master")),
                "{framework} server failed to stop gracefully: {}",
                shutdown_log
            );
        }
    }
}

#[cfg(unix)]
struct PythonServer(std::process::Child);

#[cfg(unix)]
impl Drop for PythonServer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn python_wsgi_rejects_attributes_the_server_cannot_import() {
    let project = tempfile::tempdir().unwrap();
    for application in ["demo:object.app", "demo:object.create()"] {
        let result = resolve_launch(
            &LocalFs::new(project.path()),
            PythonLaunchRequest {
                entry: None,
                module: None,
                application: Some(application),
                server: Some("wsgi"),
                args: &[],
            },
        );
        assert!(result.is_err(), "gunicorn cannot import {application}");
    }
}

#[cfg(unix)]
#[test]
fn python_bootstrap_rejects_symlinked_generated_layout() {
    let project = tempfile::tempdir().unwrap();
    let state = project.path().join("deep/generated");
    std::fs::create_dir_all(&state).unwrap();
    std::os::unix::fs::symlink("deep/generated", project.path().join(".onreza")).unwrap();
    std::fs::write(project.path().join("demo.py"), "print('project-root')\n").unwrap();
    let launch = resolve_launch(
        &LocalFs::new(project.path()),
        PythonLaunchRequest {
            entry: None,
            module: Some("demo"),
            application: None,
            server: None,
            args: &[],
        },
    )
    .unwrap()
    .unwrap();
    let error = materialize_python_entry(
        project.path(),
        &launch,
        nrz_source_bundle::PythonMinor::default(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("must not contain symlinks"));
    assert!(!state.join("python").exists());
}

#[test]
fn python_bootstrap_preserves_the_selected_build_minor_on_another_interpreter() {
    for minor in nrz_source_bundle::PythonMinor::ALL {
        let project = tempfile::tempdir().unwrap();
        for other in nrz_source_bundle::PythonMinor::ALL {
            std::fs::create_dir_all(project.path().join(other.site_packages_root())).unwrap();
            std::fs::write(
                project
                    .path()
                    .join(other.site_packages_root())
                    .join("demo.py"),
                format!("print('{}')\n", other.version()),
            )
            .unwrap();
        }
        let launch = crate::detect::python_launch::PythonLaunch {
            entry: crate::detect::python_launch::PYTHON_BOOTSTRAP_ENTRY.into(),
            args: vec!["MODULE".into(), "demo".into()],
        };
        let entry = materialize_python_entry(project.path(), &launch, minor).unwrap();
        let output = Command::new("python3")
            .args(["-I", &entry])
            .args(&launch.args)
            .current_dir(project.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            minor.version()
        );
    }
}

fn install_real_pth_wheel(project: &std::path::Path, minor: nrz_source_bundle::PythonMinor) {
    let app = "import builtins,json,sys\nfrom . import ORIGIN\ndef observed():\n return {'origin':ORIGIN,'initialized':getattr(builtins,'PTH_INIT_COUNT',0),'initializedOrigin':getattr(builtins,'PTH_INIT_ORIGIN',None),'args':sys.argv[1:]}\ndef main():\n print(json.dumps(observed()))\n return 0\ndef server_observed():\n value=observed(); value['args']=[arg for arg in value['args'] if arg.startswith('literal spaces')]\n return value\nasync def asgi(scope,receive,send):\n if scope['type']!='http': raise ValueError('no lifespan')\n await send({'type':'http.response.start','status':200,'headers':[(b'content-type',b'application/json')]})\n await send({'type':'http.response.body','body':json.dumps(server_observed()).encode()})\ndef wsgi(environ,start_response):\n start_response('200 OK',[('Content-Type','application/json')])\n return [json.dumps(server_observed()).encode()]\n";
    for base in ["onreza_pth_app", "src/onreza_pth_app"] {
        let directory = project.join(base);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("__init__.py"), "ORIGIN='SOURCE_FALLBACK'\n").unwrap();
        std::fs::write(directory.join("app.py"), app).unwrap();
        std::fs::write(
            directory.join("__main__.py"),
            "from .app import main\nmain()\n",
        )
        .unwrap();
    }
    std::fs::write(
        project.join("onreza_pth_other.py"),
        "ORIGIN='SOURCE_FALLBACK'\n",
    )
    .unwrap();
    let files = serde_json::json!({
        "pth_fixture-1.0.dist-info/WHEEL":"Wheel-Version: 1.0\nGenerator: onreza-fixture\nRoot-Is-Purelib: true\nTag: py3-none-any\n",
        "pth_fixture-1.0.dist-info/METADATA":"Metadata-Version: 2.1\nName: pth-fixture\nVersion: 1.0\n",
        "pth_fixture.pth":"./vendor/nested\nimport pth_fixture_init\n",
        "sitecustomize.py":"import builtins,json,sys,onreza_pth_app\nprint('ONREZA_SITE_CUSTOMIZE '+json.dumps({'origin':onreza_pth_app.ORIGIN,'initialized':getattr(builtins,'PTH_INIT_COUNT',0)}),file=sys.stderr)\n",
        "vendor/nested/onreza_pth_other.py":"ORIGIN='INSTALLED_WHEEL'\n",
        "pth_fixture_init.py":"import builtins,onreza_pth_app\nbuiltins.PTH_INIT_COUNT=getattr(builtins,'PTH_INIT_COUNT',0)+1\nbuiltins.PTH_INIT_ORIGIN=onreza_pth_app.ORIGIN\n",
        "vendor/nested/onreza_pth_app/__init__.py":"ORIGIN='INSTALLED_WHEEL'\n",
        "vendor/nested/onreza_pth_app/app.py":app,
        "vendor/nested/onreza_pth_app/__main__.py":"from .app import main\nmain()\n"
    });
    let wheel = project.join("pth_fixture-1.0-py3-none-any.whl");
    let generated = Command::new("python3")
        .args(["-I", "-c", "import base64,hashlib,json,sys,zipfile\nfiles=json.loads(sys.argv[2]); records=[]\nwith zipfile.ZipFile(sys.argv[1],'w') as archive:\n for name,text in files.items():\n  data=text.encode(); archive.writestr(name,data); records.append(name+',sha256='+base64.urlsafe_b64encode(hashlib.sha256(data).digest()).decode().rstrip('=')+','+str(len(data)))\n archive.writestr('pth_fixture-1.0.dist-info/RECORD','\\n'.join(records)+'\\npth_fixture-1.0.dist-info/RECORD,,\\n')\n"])
        .arg(&wheel)
        .arg(files.to_string())
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    // The installer is covered by its own qualification suite. These launch
    // tests consume the real wheel's package layout without adding an external
    // UV prerequisite to the ordinary Rust test container.
    let unpacked = Command::new("python3")
        .args([
            "-I",
            "-c",
            "import sys,zipfile; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])",
        ])
        .arg(&wheel)
        .arg(project.join(minor.site_packages_root()))
        .output()
        .unwrap();
    assert!(
        unpacked.status.success(),
        "{}",
        String::from_utf8_lossy(&unpacked.stderr)
    );
}

fn assert_python_pth_launch(mode: &str) {
    for minor in nrz_source_bundle::PythonMinor::ALL {
        let project = tempfile::Builder::new()
            .prefix("nrz pth nested-")
            .tempdir()
            .unwrap();
        install_real_pth_wheel(project.path(), minor);
        let arguments = vec![
            "literal spaces ' \" $(touch forbidden)".to_string(),
            "line\nbreak".to_string(),
        ];
        let launch = resolve_launch(
            &LocalFs::new(project.path()),
            PythonLaunchRequest {
                entry: None,
                module: (mode == "MODULE").then_some("onreza_pth_app"),
                application: (mode == "CALLABLE").then_some("onreza_pth_app.app:main"),
                server: None,
                args: &arguments,
            },
        )
        .unwrap()
        .unwrap();
        let entry = materialize_python_entry(project.path(), &launch, minor).unwrap();
        let result = Command::new("python3")
            .args(["-I", &entry])
            .args(&launch.args)
            .current_dir(project.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{mode}/{}: {}",
            minor.version(),
            String::from_utf8_lossy(&result.stderr)
        );
        let observed: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(
            observed["origin"],
            "INSTALLED_WHEEL",
            "{mode}/{}: {observed}",
            minor.version()
        );
        assert_eq!(observed["initializedOrigin"], "INSTALLED_WHEEL");
        assert_eq!(observed["initialized"], 1);
        assert_eq!(observed["args"], serde_json::json!(arguments));
        assert!(!project.path().join("forbidden").exists());
    }
}

#[test]
fn python_module_processes_real_wheel_pth_before_source_fallback() {
    assert_python_pth_launch("MODULE");
}

#[test]
fn python_callable_processes_real_wheel_pth_before_source_fallback() {
    assert_python_pth_launch("CALLABLE");
}

#[test]
fn python_script_wrapper_and_bootstrap_share_pth_initialization_once() {
    for minor in nrz_source_bundle::PythonMinor::ALL {
        for mode in ["SCRIPT", "MODULE", "CALLABLE"] {
            let project = tempfile::Builder::new()
                .prefix("nrz pth wrapper-")
                .tempdir()
                .unwrap();
            install_real_pth_wheel(project.path(), minor);
            std::fs::create_dir_all(project.path().join("scripts")).unwrap();
            std::fs::write(
                project.path().join("scripts/runner.py"),
                "from onreza_pth_app.app import main\nmain()\n",
            )
            .unwrap();
            let arguments = vec![
                "literal spaces ' \" $(touch forbidden)".to_string(),
                "line\nbreak".to_string(),
            ];
            let launch = resolve_launch(
                &LocalFs::new(project.path()),
                PythonLaunchRequest {
                    entry: (mode == "SCRIPT").then_some("scripts/runner.py"),
                    module: (mode == "MODULE").then_some("onreza_pth_app"),
                    application: (mode == "CALLABLE").then_some("onreza_pth_app.app:main"),
                    server: None,
                    args: &arguments,
                },
            )
            .unwrap()
            .unwrap();
            let entry = materialize_python_entry(project.path(), &launch, minor).unwrap();
            // A relative alias reaches the same canonical stage as the inner
            // bootstrap, whose initializer definition executes a second time.
            let stage = format!("./{}", minor.site_packages_root());
            let wrapper = nrz_runtime_artifact::python_script_launch_arguments(
                &entry,
                &stage,
                project.path().to_str().unwrap(),
            );
            let result = Command::new("python3")
                .arg("-I")
                .args(wrapper)
                .args(&launch.args)
                .current_dir(project.path())
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{mode}/{}: {}",
                minor.version(),
                String::from_utf8_lossy(&result.stderr)
            );
            let observed: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
            assert_eq!(observed["origin"], "INSTALLED_WHEEL");
            assert_eq!(observed["initializedOrigin"], "INSTALLED_WHEEL");
            assert_eq!(observed["initialized"], 1);
            assert_eq!(observed["args"], serde_json::json!(arguments));
            assert!(!project.path().join("forbidden").exists());
            let stderr = String::from_utf8(result.stderr).unwrap();
            let customizations = stderr
                .lines()
                .filter_map(|line| line.strip_prefix("ONREZA_SITE_CUSTOMIZE "))
                .collect::<Vec<_>>();
            assert_eq!(customizations.len(), 1, "{mode}: {stderr}");
            let customization: serde_json::Value = serde_json::from_str(customizations[0]).unwrap();
            assert_eq!(
                customization,
                serde_json::json!({"origin":"INSTALLED_WHEEL", "initialized":1})
            );
        }
    }
}

#[test]
fn python_site_initialization_promotes_preexisting_wheel_pth_directory() {
    let project = tempfile::tempdir().unwrap();
    let minor = nrz_source_bundle::PythonMinor::Python314;
    install_real_pth_wheel(project.path(), minor);
    std::fs::write(
        project.path().join("probe.py"),
        "import onreza_pth_other\nprint(onreza_pth_other.ORIGIN)\n",
    )
    .unwrap();
    let vendor = project
        .path()
        .join(minor.site_packages_root())
        .join("vendor/nested");
    let mut wrapper = nrz_runtime_artifact::python_script_launch_arguments(
        "probe.py",
        minor.site_packages_root(),
        project.path().to_str().unwrap(),
    );
    let code = wrapper
        .iter_mut()
        .skip_while(|arg| arg.as_str() != "-c")
        .nth(1)
        .unwrap();
    *code = format!(
        "import sys\nsys.path.insert(1,{})\n{}",
        serde_json::to_string(&vendor.to_string_lossy()).unwrap(),
        code
    );
    let result = Command::new("python3")
        .arg("-I")
        .args(wrapper)
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        String::from_utf8(result.stdout).unwrap().trim(),
        "INSTALLED_WHEEL"
    );
}

#[test]
fn python_script_wrapper_preserves_script_directory_fallback_priority() {
    let project = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(project.path().join("scripts")).unwrap();
    std::fs::create_dir_all(project.path().join("src")).unwrap();
    for (directory, origin) in [
        (".", "PROJECT_ROOT"),
        ("src", "SOURCE_LAYOUT"),
        ("scripts", "SCRIPT_DIRECTORY"),
    ] {
        std::fs::write(
            project.path().join(directory).join("helper.py"),
            format!("ORIGIN='{origin}'\n"),
        )
        .unwrap();
    }
    std::fs::write(
        project.path().join("scripts/main.py"),
        "import helper,json,sys\nprint(json.dumps({'origin':helper.ORIGIN,'args':sys.argv}))\n",
    )
    .unwrap();
    let arguments = vec!["literal spaces ' \" $(touch forbidden)".to_string()];
    let launch = resolve_launch(
        &LocalFs::new(project.path()),
        PythonLaunchRequest {
            entry: Some("scripts/main.py"),
            module: None,
            application: None,
            server: None,
            args: &arguments,
        },
    )
    .unwrap()
    .unwrap();
    let minor = nrz_source_bundle::PythonMinor::Python314;
    let entry = materialize_python_entry(project.path(), &launch, minor).unwrap();
    let direct = Command::new("python3")
        .arg(&entry)
        .args(&launch.args)
        .current_dir(project.path())
        .output()
        .unwrap();
    assert!(direct.status.success());
    let direct: serde_json::Value = serde_json::from_slice(&direct.stdout).unwrap();
    assert_eq!(direct["origin"], "SCRIPT_DIRECTORY");
    let wrapper = nrz_runtime_artifact::python_script_launch_arguments(
        &entry,
        minor.site_packages_root(),
        project.path().to_str().unwrap(),
    );
    for expected in ["SCRIPT_DIRECTORY", "INSTALLED_WHEEL"] {
        let result = Command::new("python3")
            .args(&wrapper)
            .args(&launch.args)
            .current_dir(project.path())
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let observed: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        assert_eq!(observed["origin"], expected);
        assert_eq!(observed["args"], direct["args"]);
        let stage = project.path().join(minor.site_packages_root());
        std::fs::create_dir_all(&stage).unwrap();
        std::fs::write(stage.join("helper.py"), "ORIGIN='INSTALLED_WHEEL'\n").unwrap();
    }
    assert!(!project.path().join("forbidden").exists());
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "installs real uvicorn/gunicorn wheels and executes HTTP servers"]
async fn python_servers_process_real_wheel_pth_before_source_fallback() {
    let mut observations = Vec::new();
    for minor in nrz_source_bundle::PythonMinor::ALL {
        for (server, dependency, option) in [
            ("asgi", "uvicorn==0.54.0", "--root-path"),
            ("wsgi", "gunicorn==26.2.0", "--name"),
        ] {
            let project = tempfile::Builder::new()
                .prefix("nrz pth server-")
                .tempdir()
                .unwrap();
            install_real_pth_wheel(project.path(), minor);
            let interpreter: std::path::PathBuf =
                if super::python_toolchain_tests::qualification_mode()
                    == super::python_toolchain::PythonInstallMode::PinnedPlatform
                {
                    minor.platform_interpreter().into()
                } else {
                    "python3".into()
                };
            let installed = Command::new("uv")
                .args(["--no-config", "pip", "install", "--python"])
                .arg(&interpreter)
                .args(["--target"])
                .arg(minor.site_packages_root())
                .arg(dependency)
                .current_dir(project.path())
                .output()
                .unwrap();
            assert!(
                installed.status.success(),
                "{}",
                String::from_utf8_lossy(&installed.stderr)
            );
            let literal = "literal spaces ' \" $(touch forbidden)";
            let arguments = vec![option.to_string(), literal.to_string()];
            let application = format!("onreza_pth_app.app:{server}");
            let launch = resolve_launch(
                &LocalFs::new(project.path()),
                PythonLaunchRequest {
                    entry: None,
                    module: None,
                    application: Some(&application),
                    server: Some(server),
                    args: &arguments,
                },
            )
            .unwrap()
            .unwrap();
            let entry = materialize_python_entry(project.path(), &launch, minor).unwrap();
            let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = listener.local_addr().unwrap().port();
            drop(listener);
            let log = tempfile::NamedTempFile::new().unwrap();
            let mut child = PythonServer(
                Command::new(&interpreter)
                    .args(["-I", &entry])
                    .args(&launch.args)
                    .current_dir(project.path())
                    .env("PORT", port.to_string())
                    .stdout(std::process::Stdio::null())
                    .stderr(log.reopen().unwrap())
                    .spawn()
                    .unwrap(),
            );
            let mut observed = None;
            for _ in 0..100 {
                if let Ok(response) = reqwest::get(format!("http://127.0.0.1:{port}/")).await
                    && response.status().is_success()
                {
                    observed = Some(response.json::<serde_json::Value>().await.unwrap());
                    break;
                }
                if child.0.try_wait().unwrap().is_some() {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
            let observed = observed.unwrap_or_else(|| {
                panic!(
                    "{server}/{}: {}",
                    minor.version(),
                    std::fs::read_to_string(log.path()).unwrap()
                )
            });
            assert!(
                observed["args"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|arg| arg == literal)
            );
            assert!(!project.path().join("forbidden").exists());
            super::python_toolchain_tests::qualification_artifact(
                project.path(),
                minor,
                &format!("pth-{server}"),
                &entry,
                &launch.args,
                Some(observed.clone()),
            )
            .await;
            observations.push((server, minor.version(), observed));
        }
    }
    for (_, _, observed) in &observations {
        assert_eq!(observed["origin"], "INSTALLED_WHEEL", "{observations:?}");
        assert_eq!(observed["initializedOrigin"], "INSTALLED_WHEEL");
        assert_eq!(observed["initialized"], 1);
    }
}
