use std::env;
use std::fs::File;
use std::io;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;

use ring::digest::{Context, Digest, SHA256};

struct ContextWriter(Context);

/// So we don't have to copy the whole file into memory.
impl io::Write for ContextWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

enum FileType {
    Css,
    Json,
    Png,
}

impl FileType {
    fn content_type(&self) -> &'static str {
        match self {
            Self::Css => "text/css; charset=UTF-8",
            Self::Json => "test/json; charset=UTF-8",
            Self::Png => "image/png",
        }
    }
}

struct EmbeddedFile {
    function_name: String,
    file_name: &'static str,
    file_type: FileType,
    file_path: PathBuf,
}

impl EmbeddedFile {
    fn new(file_name: &'static str, file_type: FileType) -> Self {
        let mut file_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        file_path.push("static");
        file_path.push(file_name);
        let function_name = file_name.replace('.', "_");
        EmbeddedFile {
            function_name,
            file_name,
            file_type,
            file_path,
        }
    }

    fn write_route_function(&self, f: &mut File, digest: &Digest) -> Result<(), io::Error> {
        writeln!(f, "#[get(\"/assets/{:?}/{}\")]", digest, self.file_name)?;
        writeln!(f, "fn {}() -> AssetResponse {{", self.function_name)?;

        {
            writeln!(
                f,
                "    AssetResponse::new(include_bytes!(r#\"{}\"#), \"{}\")",
                self.file_path.to_str().unwrap(),
                self.file_type.content_type()
            )?;
        }

        writeln!(f, "}}")?;
        writeln!(f)?;

        Ok(())
    }

    fn write_uri_function(&self, f: &mut File) -> Result<(), io::Error> {
        writeln!(f, "#[allow(dead_code)]")?;
        writeln!(f, "pub fn {}_uri() -> String {{", self.function_name)?;

        writeln!(f, "    uri!({}()).to_string()", self.function_name)?;

        writeln!(f, "}}")?;
        writeln!(f)?;

        Ok(())
    }

    fn write(&self, f: &mut File, digest: &Digest) -> Result<(), io::Error> {
        self.write_route_function(f, digest)?;
        self.write_uri_function(f)?;
        Ok(())
    }
}

fn rebuild_on_file_change(context: &mut ContextWriter, path: &Path) -> Result<(), io::Error> {
    println!("cargo:rerun-if-changed={}", path.canonicalize()?.to_str().unwrap());
    let mut file = File::open(path)?;
    io::copy(&mut file, context)?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let files = [
        EmbeddedFile::new("primer.css", FileType::Css),
        EmbeddedFile::new("primer.css.map", FileType::Json),
        EmbeddedFile::new("favicon.png", FileType::Png),
    ];

    // Record all inputs, including this build script.
    // We will use the contents of all files to generate a hash for the URL.
    let mut context = ContextWriter(Context::new(&SHA256));
    rebuild_on_file_change(&mut context, PathBuf::from("build.rs").as_path())?;
    for f in &files {
        rebuild_on_file_change(&mut context, f.file_path.as_path())?;
    }
    let digest = context.0.finish();

    let mut generated_file_path = PathBuf::from(env::var("OUT_DIR")?);
    generated_file_path.push("assets.rs");
    // NOTE: this file is included into src/assets.rs. So it uses some `use` statements from there.
    let mut generated_file = File::create(generated_file_path)?;

    for f in &files {
        f.write(&mut generated_file, &digest)?;
    }

    writeln!(
        generated_file,
        "pub fn mount_routes(rocket: Rocket<Build>) -> Rocket<Build> {{"
    )?;
    writeln!(
        generated_file,
        "    rocket.mount(\"/\", routes![{}])",
        &files.map(|f| f.function_name).join(", ")
    )?;
    writeln!(generated_file, "}}")?;

    Ok(())
}
