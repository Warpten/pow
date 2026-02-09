use protogen::Generator;

fn main() -> std::io::Result<()> {
    built::write_built_file()?;

    Generator::default()
        .build("C:/Users/verto/xmake/protoc.exe");

    Ok(())
}