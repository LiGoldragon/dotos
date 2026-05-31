//! Proc-macro derives for `nota-next`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStreamTwo;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Field, Fields, GenericParam,
    Generics, Ident, Index, LitStr, Variant, parse_macro_input,
};

#[proc_macro_derive(NotaDecode, attributes(nota))]
pub fn derive_nota_decode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    CodecDerive::new(input).expand_decode().into()
}

#[proc_macro_derive(NotaEncode, attributes(nota))]
pub fn derive_nota_encode(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    CodecDerive::new(input).expand_encode().into()
}

struct CodecDerive {
    input: DeriveInput,
}

impl CodecDerive {
    fn new(input: DeriveInput) -> Self {
        Self { input }
    }

    fn expand_decode(self) -> TokenStreamTwo {
        self.expand(CodecDirection::Decode)
    }

    fn expand_encode(self) -> TokenStreamTwo {
        self.expand(CodecDirection::Encode)
    }

    fn expand(self, direction: CodecDirection) -> TokenStreamTwo {
        let attributes = match ContainerNotaAttributes::from_attributes(&self.input.attrs) {
            Ok(attributes) => attributes,
            Err(error) => return error.to_compile_error(),
        };
        let name = self.input.ident;
        match self.input.data {
            Data::Struct(data) => {
                StructDerive::new(name, self.input.generics, data, direction, attributes).expand()
            }
            Data::Enum(data) => {
                EnumDerive::new(name, self.input.generics, data, direction).expand()
            }
            Data::Union(_) => Error::new_spanned(name, "Nota codec derives do not support unions")
                .to_compile_error(),
        }
    }
}

#[derive(Clone, Copy)]
enum CodecDirection {
    Decode,
    Encode,
}

impl CodecDirection {
    fn bound(self) -> TokenStreamTwo {
        match self {
            Self::Decode => quote!(::nota_next::NotaDecode),
            Self::Encode => quote!(::nota_next::NotaEncode),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct ContainerNotaAttributes {
    known_root: bool,
}

impl ContainerNotaAttributes {
    fn from_attributes(attributes: &[Attribute]) -> Result<Self, Error> {
        let mut output = Self::default();
        for attribute in attributes {
            if !attribute.path().is_ident("nota") {
                continue;
            }
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("known_root") {
                    output.known_root = true;
                    return Ok(());
                }
                Err(meta.error("unsupported nota container attribute"))
            })?;
        }
        Ok(output)
    }

    fn known_root(&self) -> bool {
        self.known_root
    }
}

#[derive(Clone, Default)]
struct FieldNotaAttributes {
    name: Option<LitStr>,
}

impl FieldNotaAttributes {
    fn from_attributes(attributes: &[Attribute]) -> Result<Self, Error> {
        let mut output = Self::default();
        for attribute in attributes {
            if !attribute.path().is_ident("nota") {
                continue;
            }
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("name") {
                    let value = meta.value()?;
                    output.name = Some(value.parse()?);
                    return Ok(());
                }
                Err(meta.error("unsupported nota field attribute"))
            })?;
        }
        Ok(output)
    }

    fn name(&self) -> Option<&LitStr> {
        self.name.as_ref()
    }
}

struct StructDerive {
    name: Ident,
    generics: Generics,
    data: DataStruct,
    direction: CodecDirection,
    attributes: ContainerNotaAttributes,
}

impl StructDerive {
    fn new(
        name: Ident,
        generics: Generics,
        data: DataStruct,
        direction: CodecDirection,
        attributes: ContainerNotaAttributes,
    ) -> Self {
        Self {
            name,
            generics,
            data,
            direction,
            attributes,
        }
    }

    fn expand(self) -> TokenStreamTwo {
        match self.direction {
            CodecDirection::Decode => self.expand_decode(),
            CodecDirection::Encode => self.expand_encode(),
        }
    }

    fn expand_decode(self) -> TokenStreamTwo {
        let name = self.name;
        let generics =
            GenericsWithCodecBound::new(self.generics, CodecDirection::Decode).generics();
        let (implementation_generics, type_generics, where_clause) = generics.split_for_impl();
        let type_name = name.to_string();
        match self.data.fields {
            Fields::Named(fields) => {
                let named_fields = fields.named;
                let field_count = named_fields.len();
                let body_fields = match named_fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| FieldDecode::new(index, field).body_named())
                    .collect::<Result<Vec<_>, _>>()
                {
                    Ok(fields) => fields,
                    Err(error) => return error.to_compile_error(),
                };
                let document_impl = if self.attributes.known_root() {
                    quote! {
                        impl #implementation_generics ::nota_next::NotaDocumentDecode for #name #type_generics #where_clause {
                            fn from_nota_document_body(body: &::nota_next::NotaDocumentBody<'_>) -> Result<Self, ::nota_next::NotaDecodeError> {
                                <Self as ::nota_next::NotaBodyDecode>::from_nota_body(body.as_body())
                            }
                        }
                    }
                } else {
                    quote! {}
                };
                quote! {
                    impl #implementation_generics ::nota_next::NotaBodyDecode for #name #type_generics #where_clause {
                        fn from_nota_body(body: &::nota_next::NotaBody<'_>) -> Result<Self, ::nota_next::NotaDecodeError> {
                            let children = body.expect_fields(#type_name, #field_count)?;
                            Ok(Self {
                                #(#body_fields,)*
                            })
                        }
                    }
                    impl #implementation_generics ::nota_next::NotaDecode for #name #type_generics #where_clause {
                        fn from_nota_block(block: &::nota_next::Block) -> Result<Self, ::nota_next::NotaDecodeError> {
                            let body = ::nota_next::NotaBlock::new(block).expect_body(
                                ::nota_next::Delimiter::Parenthesis,
                                #type_name,
                            )?;
                            <Self as ::nota_next::NotaBodyDecode>::from_nota_body(&body)
                        }
                    }
                    #document_impl
                }
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field_type = &fields.unnamed.first().expect("one field checked").ty;
                quote! {
                    impl #implementation_generics ::nota_next::NotaDecode for #name #type_generics #where_clause {
                        fn from_nota_block(block: &::nota_next::Block) -> Result<Self, ::nota_next::NotaDecodeError> {
                            Ok(Self(<#field_type as ::nota_next::NotaDecode>::from_nota_block(block)?))
                        }
                    }
                }
            }
            Fields::Unnamed(fields) => Error::new_spanned(
                fields,
                "NotaDecode supports named structs or one-field tuple newtypes",
            )
            .to_compile_error(),
            Fields::Unit => Error::new_spanned(
                name,
                "NotaDecode supports named structs or one-field tuple newtypes",
            )
            .to_compile_error(),
        }
    }

    fn expand_encode(self) -> TokenStreamTwo {
        let name = self.name;
        let generics =
            GenericsWithCodecBound::new(self.generics, CodecDirection::Encode).generics();
        let (implementation_generics, type_generics, where_clause) = generics.split_for_impl();
        match self.data.fields {
            Fields::Named(fields) => {
                let named_fields = fields.named;
                let body_fields = match named_fields
                    .iter()
                    .map(FieldEncode::body_named)
                    .collect::<Result<Vec<_>, _>>()
                {
                    Ok(fields) => fields,
                    Err(error) => return error.to_compile_error(),
                };
                let document_impl = if self.attributes.known_root() {
                    quote! {
                        impl #implementation_generics ::nota_next::NotaDocumentEncode for #name #type_generics #where_clause {
                            fn to_nota_document_body(&self) -> ::nota_next::NotaDocumentEncoding {
                                <Self as ::nota_next::NotaBodyEncode>::to_nota_body(self)
                            }
                        }
                    }
                } else {
                    quote! {}
                };
                quote! {
                    impl #implementation_generics ::nota_next::NotaBodyEncode for #name #type_generics #where_clause {
                        fn to_nota_body(&self) -> ::nota_next::NotaBodyEncoding {
                            ::nota_next::NotaBodyEncoding::new(vec![
                                #(#body_fields,)*
                            ])
                        }
                    }
                    impl #implementation_generics ::nota_next::NotaEncode for #name #type_generics #where_clause {
                        fn to_nota(&self) -> String {
                            <Self as ::nota_next::NotaBodyEncode>::to_nota_body(self)
                                .to_delimited_nota(::nota_next::Delimiter::Parenthesis)
                        }
                    }
                    #document_impl
                }
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                quote! {
                    impl #implementation_generics ::nota_next::NotaEncode for #name #type_generics #where_clause {
                        fn to_nota(&self) -> String {
                            ::nota_next::NotaEncode::to_nota(&self.0)
                        }
                    }
                }
            }
            Fields::Unnamed(fields) => Error::new_spanned(
                fields,
                "NotaEncode supports named structs or one-field tuple newtypes",
            )
            .to_compile_error(),
            Fields::Unit => Error::new_spanned(
                name,
                "NotaEncode supports named structs or one-field tuple newtypes",
            )
            .to_compile_error(),
        }
    }
}

struct FieldDecode<'field> {
    index: usize,
    field: &'field Field,
}

impl<'field> FieldDecode<'field> {
    fn new(index: usize, field: &'field Field) -> Self {
        Self { index, field }
    }

    fn body_named(&self) -> Result<TokenStreamTwo, Error> {
        let name = self.field.ident.as_ref().expect("named field");
        let field_type = &self.field.ty;
        let index = Index::from(self.index);
        let attributes = FieldNotaAttributes::from_attributes(&self.field.attrs)?;
        if let Some(body_name) = attributes.name() {
            return Ok(quote! {
                #name: <#field_type as ::nota_next::NotaNamedBodyFieldDecode>::from_nota_named_body_field(#body_name, &children[#index])?
            });
        }
        Ok(quote! {
            #name: <#field_type as ::nota_next::NotaDecode>::from_nota_block(&children[#index])?
        })
    }
}

struct FieldEncode;

impl FieldEncode {
    fn body_named(field: &Field) -> Result<TokenStreamTwo, Error> {
        let name = field.ident.as_ref().expect("named field");
        let attributes = FieldNotaAttributes::from_attributes(&field.attrs)?;
        if attributes.name().is_some() {
            return Ok(quote! {
                ::nota_next::NotaNamedBodyFieldEncode::to_nota_named_body_field(&self.#name)
            });
        }
        Ok(quote! {
            ::nota_next::NotaEncode::to_nota(&self.#name)
        })
    }
}

struct EnumDerive {
    name: Ident,
    generics: Generics,
    data: DataEnum,
    direction: CodecDirection,
}

impl EnumDerive {
    fn new(name: Ident, generics: Generics, data: DataEnum, direction: CodecDirection) -> Self {
        Self {
            name,
            generics,
            data,
            direction,
        }
    }

    fn expand(self) -> TokenStreamTwo {
        match self.direction {
            CodecDirection::Decode => self.expand_decode(),
            CodecDirection::Encode => self.expand_encode(),
        }
    }

    fn expand_decode(self) -> TokenStreamTwo {
        let name = self.name;
        let generics =
            GenericsWithCodecBound::new(self.generics, CodecDirection::Decode).generics();
        let (implementation_generics, type_generics, where_clause) = generics.split_for_impl();
        let enum_name = name.to_string();
        let unit_variants = self
            .data
            .variants
            .iter()
            .filter(|variant| matches!(variant.fields, Fields::Unit))
            .map(|variant| UnitVariantDecode::new(&name, variant).arm());
        let payload_variants = self
            .data
            .variants
            .iter()
            .filter(|variant| !matches!(variant.fields, Fields::Unit))
            .map(|variant| PayloadVariantDecode::new(&name, variant).arm());
        quote! {
            impl #implementation_generics ::nota_next::NotaBodyDecode for #name #type_generics #where_clause {
                fn from_nota_body(body: &::nota_next::NotaBody<'_>) -> Result<Self, ::nota_next::NotaDecodeError> {
                    let root_objects = body.root_objects();
                    if root_objects.len() == 1 {
                        if let Some(variant) = root_objects[0].demote_to_string() {
                            return match variant {
                                #(#unit_variants)*
                                other => Err(::nota_next::NotaDecodeError::UnknownVariant {
                                    enum_name: #enum_name,
                                    variant: other.to_owned(),
                                }),
                            };
                        }
                    }
                    let children = body.expect_fields(#enum_name, 2)?;
                    let variant = children[0].demote_to_string().ok_or(::nota_next::NotaDecodeError::ExpectedAtom {
                        type_name: "enum variant",
                    })?;
                    match variant {
                        #(#payload_variants)*
                        other => Err(::nota_next::NotaDecodeError::UnknownVariant {
                            enum_name: #enum_name,
                            variant: other.to_owned(),
                        }),
                    }
                }
            }
            impl #implementation_generics ::nota_next::NotaDecode for #name #type_generics #where_clause {
                fn from_nota_block(block: &::nota_next::Block) -> Result<Self, ::nota_next::NotaDecodeError> {
                    if block.demote_to_string().is_some() {
                        let root_objects = ::std::slice::from_ref(block);
                        let body = ::nota_next::NotaBody::new(root_objects);
                        return <Self as ::nota_next::NotaBodyDecode>::from_nota_body(&body);
                    }
                    let body = ::nota_next::NotaBlock::new(block).expect_body(
                        ::nota_next::Delimiter::Parenthesis,
                        #enum_name,
                    )?;
                    <Self as ::nota_next::NotaBodyDecode>::from_nota_body(&body)
                }
            }
        }
    }

    fn expand_encode(self) -> TokenStreamTwo {
        let name = self.name;
        let generics =
            GenericsWithCodecBound::new(self.generics, CodecDirection::Encode).generics();
        let (implementation_generics, type_generics, where_clause) = generics.split_for_impl();
        let arms = self
            .data
            .variants
            .iter()
            .map(|variant| VariantEncode::new(&name, variant).body_arm());
        quote! {
            impl #implementation_generics ::nota_next::NotaBodyEncode for #name #type_generics #where_clause {
                fn to_nota_body(&self) -> ::nota_next::NotaBodyEncoding {
                    match self {
                        #(#arms)*
                    }
                }
            }
            impl #implementation_generics ::nota_next::NotaEncode for #name #type_generics #where_clause {
                fn to_nota(&self) -> String {
                    let body = <Self as ::nota_next::NotaBodyEncode>::to_nota_body(self);
                    if body.fields().len() == 1 {
                        body.to_nota()
                    } else {
                        body.to_delimited_nota(::nota_next::Delimiter::Parenthesis)
                    }
                }
            }
        }
    }
}

struct UnitVariantDecode<'variant> {
    enum_name: &'variant Ident,
    variant: &'variant Variant,
}

impl<'variant> UnitVariantDecode<'variant> {
    fn new(enum_name: &'variant Ident, variant: &'variant Variant) -> Self {
        Self { enum_name, variant }
    }

    fn arm(&self) -> TokenStreamTwo {
        let enum_name = self.enum_name;
        let variant_name = &self.variant.ident;
        let tag = variant_name.to_string();
        quote! {
            #tag => Ok(#enum_name::#variant_name),
        }
    }
}

struct PayloadVariantDecode<'variant> {
    enum_name: &'variant Ident,
    variant: &'variant Variant,
}

impl<'variant> PayloadVariantDecode<'variant> {
    fn new(enum_name: &'variant Ident, variant: &'variant Variant) -> Self {
        Self { enum_name, variant }
    }

    fn arm(&self) -> TokenStreamTwo {
        let enum_name = self.enum_name;
        let variant_name = &self.variant.ident;
        let tag = variant_name.to_string();
        match &self.variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field_type = &fields.unnamed.first().expect("one field checked").ty;
                quote! {
                    #tag => Ok(#enum_name::#variant_name(<#field_type as ::nota_next::NotaDecode>::from_nota_block(&children[1])?)),
                }
            }
            Fields::Unnamed(fields) => {
                let field_count = fields.unnamed.len();
                let field_decode = fields.unnamed.iter().enumerate().map(|(index, field)| {
                    let field_type = &field.ty;
                    let index = Index::from(index);
                    quote! {
                        <#field_type as ::nota_next::NotaDecode>::from_nota_block(&payload_children[#index])?
                    }
                });
                quote! {
                    #tag => {
                        let payload_children = ::nota_next::NotaBlock::new(&children[1]).expect_children(
                            ::nota_next::Delimiter::Parenthesis,
                            #tag,
                            #field_count,
                        )?;
                        Ok(#enum_name::#variant_name(#(#field_decode),*))
                    }
                }
            }
            Fields::Named(fields) => Error::new_spanned(
                fields,
                "NotaDecode enum payload variants must carry unnamed fields, not named fields",
            )
            .to_compile_error(),
            Fields::Unit => {
                Error::new_spanned(variant_name, "unit variants are handled by the atom branch")
                    .to_compile_error()
            }
        }
    }
}

struct VariantEncode<'variant> {
    enum_name: &'variant Ident,
    variant: &'variant Variant,
}

impl<'variant> VariantEncode<'variant> {
    fn new(enum_name: &'variant Ident, variant: &'variant Variant) -> Self {
        Self { enum_name, variant }
    }

    fn body_arm(&self) -> TokenStreamTwo {
        let enum_name = self.enum_name;
        let variant_name = &self.variant.ident;
        let tag = variant_name.to_string();
        match &self.variant.fields {
            Fields::Unit => quote! {
                #enum_name::#variant_name => {
                    ::nota_next::NotaBodyEncoding::new(vec![#tag.to_owned()])
                }
            },
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let binding = format_ident!("payload");
                quote! {
                    #enum_name::#variant_name(#binding) => {
                        ::nota_next::NotaBodyEncoding::new(vec![
                            #tag.to_owned(),
                            ::nota_next::NotaEncode::to_nota(#binding),
                        ])
                    }
                }
            }
            Fields::Unnamed(fields) => {
                let bindings = (0..fields.unnamed.len())
                    .map(|index| format_ident!("payload_field_{}", index))
                    .collect::<Vec<_>>();
                let encoded_fields = bindings.iter().map(|binding| {
                    quote! {
                        ::nota_next::NotaEncode::to_nota(#binding)
                    }
                });
                quote! {
                    #enum_name::#variant_name(#(#bindings),*) => {
                        let payload = ::nota_next::Delimiter::Parenthesis.wrap([
                            #(#encoded_fields),*
                        ]);
                        ::nota_next::NotaBodyEncoding::new(vec![#tag.to_owned(), payload])
                    }
                }
            }
            Fields::Named(fields) => Error::new_spanned(
                fields,
                "NotaEncode enum payload variants must carry unnamed fields, not named fields",
            )
            .to_compile_error(),
        }
    }
}
struct GenericsWithCodecBound {
    generics: Generics,
    direction: CodecDirection,
}

impl GenericsWithCodecBound {
    fn new(generics: Generics, direction: CodecDirection) -> Self {
        Self {
            generics,
            direction,
        }
    }

    fn generics(mut self) -> Generics {
        let bound = self.direction.bound();
        for parameter in self.generics.params.iter_mut() {
            if let GenericParam::Type(type_parameter) = parameter {
                type_parameter.bounds.push(syn::parse_quote!(#bound));
            }
        }
        self.generics
    }
}
