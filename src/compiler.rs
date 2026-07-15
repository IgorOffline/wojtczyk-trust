use std::{
    env::consts::EXE_SUFFIX,
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SingleFileAction {
    Check,
    Build,
    Test,
}

pub struct SingleFileCompilation {
    pub output: Output,
    pub binary: Option<PathBuf>,
}

pub fn compile_single_file(
    root: &Path,
    source: &Path,
    action: SingleFileAction,
) -> io::Result<SingleFileCompilation> {
    if source.extension().and_then(|extension| extension.to_str()) != Some("rs") {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "single-file mode requires an open .rs file",
        ));
    }

    let output_dir = root.join("target").join("trust-single-file");
    fs::create_dir_all(&output_dir)?;
    let stem = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("trust-program");
    let binary = output_dir.join(format!("{stem}{EXE_SUFFIX}"));

    let mut command = Command::new("rustc");
    command.arg("--edition=2024").arg(source).current_dir(root);
    match action {
        SingleFileAction::Check => {
            command
                .args(["--emit=metadata", "--out-dir"])
                .arg(&output_dir);
        }
        SingleFileAction::Build => {
            command.arg("-g").arg("-o").arg(&binary);
        }
        SingleFileAction::Test => {
            command.args(["--test", "-g", "-o"]).arg(&binary);
        }
    }

    let output = command.output()?;
    Ok(SingleFileCompilation {
        output,
        binary: (action != SingleFileAction::Check).then_some(binary),
    })
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{SingleFileAction, compile_single_file};

    #[test]
    fn compiles_and_runs_a_file_with_a_module() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("trust-single-file-{unique}"));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("hello.rs");
        fs::write(
            &source,
            "mod message; fn main() { println!(\"{}\", message::text()); }",
        )
        .unwrap();
        fs::write(
            root.join("message.rs"),
            "pub fn text() -> &'static str { \"included\" }",
        )
        .unwrap();

        let compilation = compile_single_file(&root, &source, SingleFileAction::Build).unwrap();
        assert!(compilation.output.status.success());
        let output = Command::new(compilation.binary.unwrap()).output().unwrap();
        assert!(output.status.success());
        assert_eq!(String::from_utf8_lossy(&output.stdout), "included\n");

        let _ = fs::remove_dir_all(root);
    }
}
