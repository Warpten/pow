use protogen::Generator;

fn main() -> std::io::Result<()> {
    built::write_built_file()?;

    match Generator::default().build("C:/Users/verto/xmake/protoc.exe") {
        Ok(_) => Ok(()),
        Err(e) => panic!("{}", e),
    }
}