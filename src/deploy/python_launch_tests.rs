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
        assert_eq!(
            std::fs::read_to_string(project.path().join(entry)).unwrap(),
            super::python_launch::PYTHON_BOOTSTRAP.replace(
                "@SITE_PACKAGES_ROOT@",
                nrz_source_bundle::PythonMinor::default().site_packages_root()
            )
        );
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
        "[project]\nname='demo'\n[project.scripts]\ndemo='demo:main'\n",
        "[tool.poetry]\nname='demo'\n[tool.poetry.scripts]\ndemo={reference='demo:main',type='console'}\n",
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
