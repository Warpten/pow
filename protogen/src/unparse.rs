use syn::Item;

trait Unparse {
    fn unparse(&self, str: &mut String);
}
/*
impl Unparse for syn::Item {
    fn unparse(&self, str: &mut String) {
        match self {
            Item::Const(this) => this.unparse(str),
            Item::Enum(this) => this.unparse(str),
            Item::ExternCrate(this) => this.unparse(str),
            Item::Fn(this) => this.unparse(str),
            Item::ForeignMod(this) => this.unparse(str),
            Item::Impl(this) => this.unparse(str),
            Item::Macro(this) => this.unparse(str),
            Item::Mod(this) => this.unparse(str),
            Item::Static(this) => this.unparse(str),
            Item::Struct(this) => this.unparse(str),
            Item::Trait(this) => this.unparse(str),
            Item::TraitAlias(this) => this.unparse(str),
            Item::Type(this) => this.unparse(str),
            Item::Union(this) => this.unparse(str),
            Item::Use(this) => this.unparse(str),
            Item::Verbatim(this) => this.unparse(str),
        }
    }
}

impl Unparse for syn::ItemEnum {
    fn unparse(&self, str: &mut String) {
        todo!()
    }
}*/