//! Proc-macro derives for `nota`.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStreamTwo;
use quote::{format_ident, quote};
use syn::{
    Attribute, Data, DataEnum, DataStruct, DeriveInput, Error, Field, Fields, GenericParam,
    Generics, Ident, Index, LitInt, LitStr, Variant, parse_macro_input,
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

#[proc_macro_derive(StructuralMacroNode, attributes(shape))]
pub fn derive_structural_macro_node(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    StructuralDerive::new(input).expand().into()
}

#[proc_macro_derive(NotaDecodeTraced, attributes(nota))]
pub fn derive_nota_decode_traced(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    TracedDerive::new(input).expand().into()
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
            Self::Decode => quote!(::nota::NotaDecode),
            Self::Encode => quote!(::nota::NotaEncode),
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
                        impl #implementation_generics ::nota::NotaDocumentDecode for #name #type_generics #where_clause {
                            fn from_nota_document_body(body: &::nota::NotaDocumentBody<'_>) -> ::std::result::Result<Self, ::nota::NotaDecodeError> {
                                Self::from_body_objects(body.root_objects())
                            }
                        }
                    }
                } else {
                    quote! {}
                };
                quote! {
                    impl #implementation_generics #name #type_generics #where_clause {
                        pub fn from_body_objects(objects: &[::nota::Block]) -> ::std::result::Result<Self, ::nota::NotaDecodeError> {
                            if objects.len() != #field_count {
                                return Err(::nota::NotaDecodeError::ExpectedRootCount {
                                    type_name: #type_name,
                                    expected: #field_count,
                                    found: objects.len(),
                                });
                            }
                            let children = objects;
                            Ok(Self {
                                #(#body_fields,)*
                            })
                        }
                    }
                    impl #implementation_generics ::nota::NotaBodyDecode for #name #type_generics #where_clause {
                        fn from_nota_body(body: &::nota::NotaBody<'_>) -> ::std::result::Result<Self, ::nota::NotaDecodeError> {
                            Self::from_body_objects(body.root_objects())
                        }
                    }
                    impl #implementation_generics ::nota::NotaDecode for #name #type_generics #where_clause {
                        fn from_nota_block(block: &::nota::Block) -> ::std::result::Result<Self, ::nota::NotaDecodeError> {
                            let body = ::nota::NotaBlock::new(block).expect_body(
                                ::nota::Delimiter::Brace,
                                #type_name,
                            )?;
                            <Self as ::nota::NotaBodyDecode>::from_nota_body(&body)
                        }
                    }
                    #document_impl
                }
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field_type = &fields.unnamed.first().expect("one field checked").ty;
                quote! {
                    impl #implementation_generics ::nota::NotaDecode for #name #type_generics #where_clause {
                        fn from_nota_block(block: &::nota::Block) -> ::std::result::Result<Self, ::nota::NotaDecodeError> {
                            Ok(Self(<#field_type as ::nota::NotaDecode>::from_nota_block(block)?))
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
                    .map(|field| FieldEncode::new(field).body_named())
                    .collect::<Result<Vec<_>, _>>()
                {
                    Ok(fields) => fields,
                    Err(error) => return error.to_compile_error(),
                };
                let document_impl = if self.attributes.known_root() {
                    quote! {
                        impl #implementation_generics ::nota::NotaDocumentEncode for #name #type_generics #where_clause {
                            fn to_nota_document_body(&self) -> ::nota::NotaDocumentEncoding {
                                <Self as ::nota::NotaBodyEncode>::to_nota_body(self)
                            }
                        }
                    }
                } else {
                    quote! {}
                };
                quote! {
                    impl #implementation_generics ::nota::NotaBodyEncode for #name #type_generics #where_clause {
                        fn to_nota_body(&self) -> ::nota::NotaBodyEncoding {
                            ::nota::NotaBodyEncoding::new(vec![
                                #(#body_fields,)*
                            ])
                        }
                    }
                    impl #implementation_generics ::nota::NotaEncode for #name #type_generics #where_clause {
                        fn to_nota(&self) -> String {
                            <Self as ::nota::NotaBodyEncode>::to_nota_body(self)
                                .to_delimited_nota(::nota::Delimiter::Brace)
                        }
                    }
                    #document_impl
                }
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                quote! {
                    impl #implementation_generics ::nota::NotaEncode for #name #type_generics #where_clause {
                        fn to_nota(&self) -> String {
                            ::nota::NotaEncode::to_nota(&self.0)
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
                #name: <#field_type as ::nota::NotaNamedBodyFieldDecode>::from_nota_named_body_field(#body_name, &children[#index])?
            });
        }
        Ok(quote! {
            #name: <#field_type as ::nota::NotaDecode>::from_nota_block(&children[#index])?
        })
    }
}

struct FieldEncode<'field> {
    field: &'field Field,
}

impl<'field> FieldEncode<'field> {
    fn new(field: &'field Field) -> Self {
        Self { field }
    }

    fn body_named(&self) -> Result<TokenStreamTwo, Error> {
        let name = self.field.ident.as_ref().expect("named field");
        let attributes = FieldNotaAttributes::from_attributes(&self.field.attrs)?;
        if attributes.name().is_some() {
            return Ok(quote! {
                ::nota::NotaNamedBodyFieldEncode::to_nota_named_body_field(&self.#name)
            });
        }
        Ok(quote! {
            ::nota::NotaEncode::to_nota(&self.#name)
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
            impl #implementation_generics ::nota::NotaDecode for #name #type_generics #where_clause {
                fn from_nota_block(block: &::nota::Block) -> ::std::result::Result<Self, ::nota::NotaDecodeError> {
                    // A unit variant is a bare atom; a data variant is the
                    // dotted application `Variant.payload`, whose head atom
                    // names the variant and whose payload carries the fields.
                    if let Some(variant) = block.atom().map(|atom| atom.text()) {
                        return match variant {
                            #(#unit_variants)*
                            other => Err(::nota::NotaDecodeError::UnknownVariant {
                                enum_name: #enum_name,
                                variant: other.to_owned(),
                            }),
                        };
                    }
                    let (head, payload) = block.as_application().ok_or(::nota::NotaDecodeError::ExpectedDelimited {
                        type_name: #enum_name,
                        delimiter: "unit-variant atom or Variant.payload application",
                    })?;
                    let variant = head.demote_to_string().ok_or(::nota::NotaDecodeError::ExpectedAtom {
                        type_name: "enum variant",
                    })?;
                    match variant {
                        #(#payload_variants)*
                        other => Err(::nota::NotaDecodeError::UnknownVariant {
                            enum_name: #enum_name,
                            variant: other.to_owned(),
                        }),
                    }
                }
            }
            impl #implementation_generics ::nota::NotaBodyDecode for #name #type_generics #where_clause {
                fn from_nota_body(body: &::nota::NotaBody<'_>) -> ::std::result::Result<Self, ::nota::NotaDecodeError> {
                    let objects = body.expect_fields(#enum_name, 1)?;
                    <Self as ::nota::NotaDecode>::from_nota_block(&objects[0])
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
            .map(|variant| VariantEncode::new(&name, variant).nota_arm());
        quote! {
            impl #implementation_generics ::nota::NotaEncode for #name #type_generics #where_clause {
                fn to_nota(&self) -> String {
                    match self {
                        #(#arms)*
                    }
                }
            }
            impl #implementation_generics ::nota::NotaBodyEncode for #name #type_generics #where_clause {
                fn to_nota_body(&self) -> ::nota::NotaBodyEncoding {
                    ::nota::NotaBodyEncoding::new(vec![::nota::NotaEncode::to_nota(self)])
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
                    #tag => Ok(#enum_name::#variant_name(<#field_type as ::nota::NotaDecode>::from_nota_block(payload)?)),
                }
            }
            Fields::Unnamed(fields) => {
                let field_count = fields.unnamed.len();
                let field_decode = fields.unnamed.iter().enumerate().map(|(index, field)| {
                    let field_type = &field.ty;
                    let index = Index::from(index);
                    quote! {
                        <#field_type as ::nota::NotaDecode>::from_nota_block(&payload_children[#index])?
                    }
                });
                quote! {
                    #tag => {
                        let payload_children = ::nota::NotaBlock::new(payload).expect_children(
                            ::nota::Delimiter::Brace,
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

    fn nota_arm(&self) -> TokenStreamTwo {
        let enum_name = self.enum_name;
        let variant_name = &self.variant.ident;
        let tag = variant_name.to_string();
        match &self.variant.fields {
            // A unit variant is a bare atom.
            Fields::Unit => quote! {
                #enum_name::#variant_name => #tag.to_owned(),
            },
            // A single-field variant dots its tag onto the payload's own
            // canonical form: `Some.42`, `Variant.(a b)`, `Variant.[ … ]`.
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let binding = format_ident!("payload");
                quote! {
                    #enum_name::#variant_name(#binding) => {
                        ::std::format!("{}.{}", #tag, ::nota::NotaEncode::to_nota(#binding))
                    }
                }
            }
            // A multi-field variant dots its tag onto a brace record of the
            // positional field values: `Variant.{f0 f1}`.
            Fields::Unnamed(fields) => {
                let bindings = (0..fields.unnamed.len())
                    .map(|index| format_ident!("payload_field_{}", index))
                    .collect::<Vec<_>>();
                let encoded_fields = bindings.iter().map(|binding| {
                    quote! {
                        ::nota::NotaEncode::to_nota(#binding)
                    }
                });
                quote! {
                    #enum_name::#variant_name(#(#bindings),*) => {
                        let payload = ::nota::Delimiter::Brace.wrap([
                            #(#encoded_fields),*
                        ]);
                        ::std::format!("{}.{}", #tag, payload)
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

struct StructuralDerive {
    input: DeriveInput,
}

impl StructuralDerive {
    fn new(input: DeriveInput) -> Self {
        Self { input }
    }

    fn expand(self) -> TokenStreamTwo {
        let DeriveInput {
            ident: name,
            generics,
            data,
            ..
        } = self.input;
        let node_name = name.to_string();
        let data = match data {
            Data::Enum(data) => data,
            Data::Struct(_) | Data::Union(_) => {
                return Error::new_spanned(&name, "StructuralMacroNode supports enums only")
                    .to_compile_error();
            }
        };
        let variants = match data
            .variants
            .iter()
            .map(|variant| StructuralVariantDerive::parse(&name, &node_name, variant))
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(variants) => variants,
            Err(error) => return error.to_compile_error(),
        };
        let structural_variants = variants.iter().map(StructuralVariantDerive::variant_value);
        let direct_decode_arms = variants
            .iter()
            .map(StructuralVariantDerive::direct_decode_arm);
        let encode_arms = variants.iter().map(StructuralVariantDerive::encode_arm);
        let (implementation_generics, type_generics, where_clause) = generics.split_for_impl();
        quote! {
            impl #implementation_generics ::nota::StructuralMacroNode for #name #type_generics #where_clause {
                type Error = ::nota::StructuralMacroNodeError;

                fn structural_position() -> ::nota::PositionPredicate {
                    ::nota::PositionPredicate::named(#node_name)
                }

                fn structural_variants() -> Vec<::nota::StructuralVariant> {
                    vec![
                        #(#structural_variants,)*
                    ]
                }

                fn from_structural_block(block: &::nota::Block) -> ::std::result::Result<Self, ::nota::StructuralMacroError<Self::Error>> {
                    let variants =
                        ::nota::StructuralVariantSet::new(Self::structural_position(), Self::structural_variants())
                            .map_err(::nota::StructuralMacroError::Dispatch)?;
                    #(#direct_decode_arms)*
                    let candidate = ::nota::MacroCandidate::from_block(Self::structural_position(), block);
                    match variants.dispatch(&candidate) {
                        Ok(matched) => Err(::nota::StructuralMacroError::MatchedNode(
                            ::nota::StructuralMacroNodeError::UnexpectedVariant {
                                node: #node_name,
                                variant: matched.macro_name().to_owned(),
                            }
                        )),
                        Err(error) => Err(::nota::StructuralMacroError::Dispatch(error)),
                    }
                }

                fn from_structural_candidate(candidate: ::nota::MacroCandidate<'_>) -> ::std::result::Result<Self, ::nota::StructuralMacroError<Self::Error>> {
                    if let [block] = candidate.blocks() {
                        return Self::from_structural_block(block);
                    }
                    let variants =
                        ::nota::StructuralVariantSet::new(Self::structural_position(), Self::structural_variants())
                            .map_err(::nota::StructuralMacroError::Dispatch)?;
                    let matched = variants
                        .dispatch(&candidate)
                        .map_err(::nota::StructuralMacroError::Dispatch)?;
                    Err(::nota::StructuralMacroError::MatchedNode(
                        ::nota::StructuralMacroNodeError::UnexpectedVariant {
                            node: #node_name,
                            variant: matched.macro_name().to_owned(),
                        }
                    ))
                }

                fn to_structural_nota(&self) -> String {
                    match self {
                        #(#encode_arms)*
                    }
                }
            }
        }
    }
}

struct StructuralVariantDerive {
    enum_name: Ident,
    node_name: String,
    variant_name: Ident,
    field_types: Vec<syn::Type>,
    shape: StructuralVariantShape,
}

impl StructuralVariantDerive {
    fn parse(enum_name: &Ident, node_name: &str, variant: &Variant) -> Result<Self, Error> {
        let field_types = match &variant.fields {
            Fields::Unit => Vec::new(),
            Fields::Unnamed(fields) => fields
                .unnamed
                .iter()
                .map(|field| field.ty.clone())
                .collect(),
            Fields::Named(fields) => {
                return Err(Error::new_spanned(
                    fields,
                    "StructuralMacroNode variants carry unnamed fields, not named fields",
                ));
            }
        };
        let shape = StructuralVariantShape::parse(variant)?;
        shape.check_field_count(variant, field_types.len())?;
        Ok(Self {
            enum_name: enum_name.clone(),
            node_name: node_name.to_owned(),
            variant_name: variant.ident.clone(),
            field_types,
            shape,
        })
    }

    fn variant_value(&self) -> TokenStreamTwo {
        let variant_name = self.variant_name.to_string();
        let expected = self.shape.expected_text(&variant_name);
        self.shape.variant_value(&variant_name, &expected)
    }

    fn direct_decode_arm(&self) -> TokenStreamTwo {
        let variant_text = self.variant_name.to_string();
        let condition = self.shape.direct_match_condition();
        let constructor = self.direct_decode_constructor(&variant_text);
        quote! {
            if #condition {
                return (|| -> ::std::result::Result<Self, Self::Error> {
                    Ok(#constructor)
                })()
                .map_err(::nota::StructuralMacroError::MatchedNode);
            }
        }
    }

    fn direct_decode_constructor(&self, variant_text: &str) -> TokenStreamTwo {
        let enum_name = &self.enum_name;
        let variant_name = &self.variant_name;
        if self.field_types.is_empty() {
            return quote!(#enum_name::#variant_name);
        }
        let node_name = &self.node_name;
        if matches!(self.shape, StructuralVariantShape::HeadedBody { .. }) {
            let field_type = &self.field_types[0];
            return quote! {
                #enum_name::#variant_name({
                    let body_blocks: ::std::vec::Vec<&::nota::Block> =
                        block.root_objects().iter().skip(1).collect();
                    <#field_type as ::nota::StructuralMacroNode>::from_structural_candidate(
                        ::nota::MacroCandidate::new(
                            <#field_type as ::nota::StructuralMacroNode>::structural_position(),
                            body_blocks,
                        ),
                    )
                    .map_err(|error| ::nota::StructuralMacroNodeError::Field {
                        node: #node_name,
                        variant: #variant_text,
                        field: 0usize,
                        error: error.to_string(),
                    })?
                })
            };
        }
        if matches!(self.shape, StructuralVariantShape::PascalHeadBody) {
            let head_type = &self.field_types[0];
            let tail_type = &self.field_types[1];
            return quote! {
                #enum_name::#variant_name(
                    {
                        let head_block = block.root_object_at(0).ok_or(
                            ::nota::StructuralMacroNodeError::MissingCapture {
                                node: #node_name,
                                variant: #variant_text,
                                capture: "field_0",
                            },
                        )?;
                        <#head_type as ::nota::StructuralMacroNode>::from_structural_block(head_block)
                            .map_err(|error| ::nota::StructuralMacroNodeError::Field {
                                node: #node_name,
                                variant: #variant_text,
                                field: 0usize,
                                error: error.to_string(),
                            })?
                    },
                    {
                        let body_blocks: ::std::vec::Vec<&::nota::Block> =
                            block.root_objects().iter().skip(1).collect();
                        <#tail_type as ::nota::StructuralMacroNode>::from_structural_candidate(
                            ::nota::MacroCandidate::new(
                                <#tail_type as ::nota::StructuralMacroNode>::structural_position(),
                                body_blocks,
                            ),
                        )
                        .map_err(|error| ::nota::StructuralMacroNodeError::Field {
                            node: #node_name,
                            variant: #variant_text,
                            field: 1usize,
                            error: error.to_string(),
                        })?
                    },
                )
            };
        }
        if matches!(self.shape, StructuralVariantShape::HeadedAtom { .. }) {
            let field_type = &self.field_types[0];
            return quote! {
                #enum_name::#variant_name({
                    let atom_text = block
                        .root_object_at(1usize)
                        .and_then(::nota::Block::demote_to_string)
                        .ok_or(::nota::StructuralMacroNodeError::MissingSlot {
                            node: #node_name,
                            variant: #variant_text,
                            capture: "atom",
                            slot: 0usize,
                        })?;
                    <#field_type as ::std::str::FromStr>::from_str(atom_text)
                        .map_err(|error| ::nota::StructuralMacroNodeError::Field {
                            node: #node_name,
                            variant: #variant_text,
                            field: 0usize,
                            error: error.to_string(),
                        })?
                })
            };
        }
        let field_decodes = self
            .field_types
            .iter()
            .enumerate()
            .map(|(field_index, field_type)| {
                let block = self
                    .shape
                    .direct_field_block(field_index, node_name, variant_text);
                quote! {
                    <#field_type as ::nota::StructuralMacroNode>::from_structural_block(#block)
                        .map_err(|error| ::nota::StructuralMacroNodeError::Field {
                            node: #node_name,
                            variant: #variant_text,
                            field: #field_index,
                            error: error.to_string(),
                        })?
                }
            });
        quote!(#enum_name::#variant_name(#(#field_decodes),*))
    }

    fn encode_arm(&self) -> TokenStreamTwo {
        let enum_name = &self.enum_name;
        let variant_name = &self.variant_name;
        let bindings = (0..self.field_types.len())
            .map(|index| format_ident!("field_{}", index))
            .collect::<Vec<_>>();
        let pattern = if bindings.is_empty() {
            quote!(#enum_name::#variant_name)
        } else {
            quote!(#enum_name::#variant_name(#(#bindings),*))
        };
        let body = self.shape.encode_body(&bindings);
        quote! {
            #pattern => #body,
        }
    }
}

enum StructuralVariantShape {
    PascalAtom,
    Keyword { keyword: String },
    Headed { head: String, arity: usize },
    HeadedAtom { head: String },
    HeadedBody { head: String },
    PascalHead { arity: usize },
    PascalHeadBody,
}

impl StructuralVariantShape {
    fn parse(variant: &Variant) -> Result<Self, Error> {
        let mut pascal_atom = false;
        let mut pascal_head = false;
        let mut head: Option<String> = None;
        let mut keyword: Option<String> = None;
        let mut arity: Option<usize> = None;
        let mut body = false;
        let mut atom = false;
        let mut found = false;
        for attribute in &variant.attrs {
            if !attribute.path().is_ident("shape") {
                continue;
            }
            found = true;
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("pascal_atom") {
                    pascal_atom = true;
                    return Ok(());
                }
                if meta.path.is_ident("pascal_head") {
                    pascal_head = true;
                    return Ok(());
                }
                if meta.path.is_ident("head") {
                    let value: LitStr = meta.value()?.parse()?;
                    head = Some(value.value());
                    return Ok(());
                }
                if meta.path.is_ident("keyword") {
                    let value: LitStr = meta.value()?.parse()?;
                    keyword = Some(value.value());
                    return Ok(());
                }
                if meta.path.is_ident("arity") {
                    let value: LitInt = meta.value()?.parse()?;
                    arity = Some(value.base10_parse()?);
                    return Ok(());
                }
                if meta.path.is_ident("body") {
                    body = true;
                    return Ok(());
                }
                if meta.path.is_ident("atom") {
                    atom = true;
                    return Ok(());
                }
                Err(meta.error("unsupported shape attribute"))
            })?;
        }
        if !found {
            return Err(Error::new_spanned(
                variant,
                "structural macro node variant needs a #[shape(...)] attribute",
            ));
        }
        if pascal_atom {
            return Ok(Self::PascalAtom);
        }
        if let Some(keyword) = keyword {
            return Ok(Self::Keyword { keyword });
        }
        if let Some(head) = head {
            if [body, atom, arity.is_some()]
                .into_iter()
                .filter(|set| *set)
                .count()
                > 1
            {
                return Err(Error::new_spanned(
                    variant,
                    "head shape takes exactly one of arity = N, body, or atom",
                ));
            }
            if body {
                return Ok(Self::HeadedBody { head });
            }
            if atom {
                return Ok(Self::HeadedAtom { head });
            }
            let arity = arity.ok_or_else(|| {
                Error::new_spanned(variant, "head shape needs arity = N, body, or atom")
            })?;
            return Ok(Self::Headed { head, arity });
        }
        if atom {
            return Err(Error::new_spanned(
                variant,
                "atom shape needs head = \"...\"",
            ));
        }
        if pascal_head {
            if body {
                if arity.is_some() {
                    return Err(Error::new_spanned(
                        variant,
                        "pascal_head shape takes either arity = N or body, not both",
                    ));
                }
                return Ok(Self::PascalHeadBody);
            }
            let arity = arity.ok_or_else(|| {
                Error::new_spanned(variant, "pascal_head shape needs arity = N or body")
            })?;
            return Ok(Self::PascalHead { arity });
        }
        if body {
            return Err(Error::new_spanned(
                variant,
                "body shape needs head = \"...\" or pascal_head",
            ));
        }
        Err(Error::new_spanned(
            variant,
            "shape must be pascal_atom, keyword = \"...\", head = \"...\" with arity, body, or atom, or pascal_head with arity",
        ))
    }

    fn check_field_count(&self, variant: &Variant, fields: usize) -> Result<(), Error> {
        let expected = match self {
            Self::PascalAtom => 1,
            Self::Keyword { .. } => 0,
            Self::Headed { arity, .. } => arity.saturating_sub(1),
            Self::HeadedAtom { .. } => 1,
            Self::HeadedBody { .. } => 1,
            Self::PascalHead { arity } => *arity,
            Self::PascalHeadBody => 2,
        };
        if fields != expected {
            return Err(Error::new_spanned(
                variant,
                format!("this shape expects {expected} field(s); the variant carries {fields}"),
            ));
        }
        Ok(())
    }

    fn variant_value(&self, variant_name: &str, expected: &str) -> TokenStreamTwo {
        match self {
            Self::PascalAtom => quote! {
                ::nota::BlockShape::pascal_atom(Some(::nota::CaptureName::new("field_0")))
                    .into_structural_variant(#variant_name, #expected)
            },
            Self::Keyword { keyword } => quote! {
                ::nota::BlockShape::literal(#keyword)
                    .into_structural_variant(#variant_name, #expected)
            },
            Self::Headed { head, arity } => {
                let arity = *arity as u64;
                quote! {
                    ::nota::BlockShape::headed_parenthesis(
                        #head,
                        ::nota::MacroObjectCount::Exact(#arity),
                        Some(::nota::CaptureName::new("signature")),
                    )
                    .into_structural_variant(#variant_name, #expected)
                }
            }
            Self::HeadedAtom { head } => quote! {
                ::nota::BlockShape::headed_parenthesis(
                    #head,
                    ::nota::MacroObjectCount::Exact(2),
                    Some(::nota::CaptureName::new("signature")),
                )
                .into_structural_variant(#variant_name, #expected)
            },
            Self::HeadedBody { head } => quote! {
                ::nota::BlockShape::headed_parenthesis(
                    #head,
                    ::nota::MacroObjectCount::Any,
                    Some(::nota::CaptureName::new("signature")),
                )
                .into_structural_variant(#variant_name, #expected)
            },
            Self::PascalHead { arity } => {
                let arity = *arity as u64;
                quote! {
                    ::nota::BlockShape::pascal_headed_parenthesis(
                        ::nota::MacroObjectCount::Exact(#arity),
                        ::nota::CaptureName::new("field_0"),
                        Some(::nota::CaptureName::new("signature")),
                    )
                    .into_structural_variant(#variant_name, #expected)
                }
            }
            Self::PascalHeadBody => quote! {
                ::nota::BlockShape::pascal_headed_parenthesis(
                    ::nota::MacroObjectCount::Any,
                    ::nota::CaptureName::new("field_0"),
                    Some(::nota::CaptureName::new("signature")),
                )
                .into_structural_variant(#variant_name, #expected)
            },
        }
    }

    fn expected_text(&self, variant_name: &str) -> String {
        match self {
            Self::PascalAtom => format!("{variant_name}: PascalCase atom"),
            Self::Keyword { keyword } => format!("{variant_name}: literal {keyword} atom"),
            Self::Headed { head, arity } => {
                format!("{variant_name}: parenthesized {head} head with arity {arity}")
            }
            Self::HeadedAtom { head } => {
                format!("{variant_name}: parenthesized {head} head with one atom field")
            }
            Self::HeadedBody { head } => {
                format!("{variant_name}: parenthesized {head} head with body objects")
            }
            Self::PascalHead { arity } => {
                format!("{variant_name}: parenthesized PascalCase head with arity {arity}")
            }
            Self::PascalHeadBody => {
                format!("{variant_name}: parenthesized PascalCase head with body objects")
            }
        }
    }

    fn direct_match_condition(&self) -> TokenStreamTwo {
        match self {
            Self::PascalAtom => quote!(block.qualifies_as_pascal_case_symbol()),
            Self::Keyword { keyword } => quote!(block.demote_to_string() == Some(#keyword)),
            Self::Headed { head, arity } => {
                let arity = *arity;
                quote! {
                    block.is_parenthesis()
                        && block.holds_root_objects() == #arity
                        && block.root_object_at(0)
                            .and_then(|root| root.demote_to_string())
                            == Some(#head)
                }
            }
            Self::HeadedAtom { head } => quote! {
                block.is_parenthesis()
                    && block.holds_root_objects() == 2usize
                    && block.root_object_at(0)
                        .and_then(|root| root.demote_to_string())
                        == Some(#head)
            },
            Self::HeadedBody { head } => quote! {
                block.is_parenthesis()
                    && block.root_object_at(0)
                        .and_then(|root| root.demote_to_string())
                        == Some(#head)
            },
            Self::PascalHead { arity } => {
                let arity = *arity;
                quote! {
                    block.is_parenthesis()
                        && block.holds_root_objects() == #arity
                        && block.root_object_at(0)
                            .is_some_and(|root| root.qualifies_as_pascal_case_symbol())
                }
            }
            Self::PascalHeadBody => quote! {
                block.is_parenthesis()
                    && block.root_object_at(0)
                        .is_some_and(|root| root.qualifies_as_pascal_case_symbol())
            },
        }
    }

    fn direct_field_block(
        &self,
        field_index: usize,
        node_name: &str,
        variant_name: &str,
    ) -> TokenStreamTwo {
        match self {
            Self::PascalAtom => quote!(block),
            Self::Keyword { .. } => quote!(block),
            Self::HeadedAtom { .. } => {
                unreachable!("atom shape decodes its leaf field through from_str")
            }
            Self::HeadedBody { .. } => {
                unreachable!("body shape decodes through from_structural_candidate")
            }
            Self::PascalHeadBody => {
                unreachable!("pascal_head body shape decodes through from_structural_candidate")
            }
            Self::Headed { .. } => {
                let child_index = field_index + 1;
                quote! {
                    block.root_object_at(#child_index)
                        .ok_or(::nota::StructuralMacroNodeError::MissingSlot {
                            node: #node_name,
                            variant: #variant_name,
                            capture: "arguments",
                            slot: #field_index,
                        })?
                }
            }
            Self::PascalHead { .. } if field_index == 0 => quote! {
                block.root_object_at(0)
                    .ok_or(::nota::StructuralMacroNodeError::MissingCapture {
                        node: #node_name,
                        variant: #variant_name,
                        capture: "field_0",
                    })?
            },
            Self::PascalHead { .. } => {
                let child_index = field_index;
                let slot = field_index - 1;
                quote! {
                    block.root_object_at(#child_index)
                        .ok_or(::nota::StructuralMacroNodeError::MissingSlot {
                            node: #node_name,
                            variant: #variant_name,
                            capture: "arguments",
                            slot: #slot,
                        })?
                }
            }
        }
    }

    fn encode_body(&self, bindings: &[Ident]) -> TokenStreamTwo {
        match self {
            Self::PascalAtom => {
                let only = &bindings[0];
                quote!(::nota::StructuralMacroNode::to_structural_nota(#only))
            }
            Self::Keyword { keyword } => quote!(#keyword.to_owned()),
            Self::HeadedAtom { head } => {
                let only = &bindings[0];
                quote!(format!("({} {})", #head, #only))
            }
            Self::HeadedBody { head } => {
                let only = &bindings[0];
                quote! {
                    {
                        let body = ::nota::StructuralMacroNode::to_structural_nota(#only);
                        if body.is_empty() {
                            format!("({})", #head)
                        } else {
                            format!("({} {})", #head, body)
                        }
                    }
                }
            }
            Self::Headed { head, .. } => {
                let format_literal = LitStr::new(
                    &self.headed_format(head, bindings.len()),
                    proc_macro2::Span::call_site(),
                );
                let arguments = bindings.iter().map(
                    |binding| quote!(::nota::StructuralMacroNode::to_structural_nota(#binding)),
                );
                quote!(format!(#format_literal, #(#arguments),*))
            }
            Self::PascalHead { .. } => {
                let format_literal = LitStr::new(
                    &self.pascal_head_format(bindings.len()),
                    proc_macro2::Span::call_site(),
                );
                let arguments = bindings.iter().map(
                    |binding| quote!(::nota::StructuralMacroNode::to_structural_nota(#binding)),
                );
                quote!(format!(#format_literal, #(#arguments),*))
            }
            Self::PascalHeadBody => {
                let head = &bindings[0];
                let tail = &bindings[1];
                quote! {
                    {
                        let head = ::nota::StructuralMacroNode::to_structural_nota(#head);
                        let body = ::nota::StructuralMacroNode::to_structural_nota(#tail);
                        if body.is_empty() {
                            format!("({})", head)
                        } else {
                            format!("({} {})", head, body)
                        }
                    }
                }
            }
        }
    }

    fn headed_format(&self, head: &str, field_count: usize) -> String {
        let mut output = String::from("(");
        output.push_str(head);
        for _ in 0..field_count {
            output.push_str(" {}");
        }
        output.push(')');
        output
    }

    fn pascal_head_format(&self, field_count: usize) -> String {
        let placeholders = vec!["{}"; field_count].join(" ");
        format!("({placeholders})")
    }
}

/// Emits `NotaDecodeTraced` alongside the ordinary `NotaDecode`: the same
/// type-directed traversal, but each step also records the reference the
/// decoder expected at that position. The trace is captured from the very path
/// that validates the value — there is no second parser and no hand-written
/// schema walk.
struct TracedDerive {
    input: DeriveInput,
}

impl TracedDerive {
    fn new(input: DeriveInput) -> Self {
        Self { input }
    }

    fn expand(self) -> TokenStreamTwo {
        let name = self.input.ident;
        let generics = GenericsWithTracedBound::new(self.input.generics).generics();
        match self.input.data {
            Data::Struct(data) => StructTrace::new(name, generics, data).expand(),
            Data::Enum(data) => EnumTrace::new(name, generics, data).expand(),
            Data::Union(_) => Error::new_spanned(name, "NotaDecodeTraced does not support unions")
                .to_compile_error(),
        }
    }
}

struct StructTrace {
    name: Ident,
    generics: Generics,
    data: DataStruct,
}

impl StructTrace {
    fn new(name: Ident, generics: Generics, data: DataStruct) -> Self {
        Self {
            name,
            generics,
            data,
        }
    }

    fn expand(self) -> TokenStreamTwo {
        let name = self.name;
        let type_name = name.to_string();
        let (implementation_generics, type_generics, where_clause) = self.generics.split_for_impl();
        match self.data.fields {
            Fields::Named(fields) => {
                let named_fields = fields.named;
                let field_count = named_fields.len();
                let field_steps = named_fields
                    .iter()
                    .enumerate()
                    .map(|(index, field)| TracedFieldStep::new(index, field).body_named())
                    .collect::<Vec<_>>();
                quote! {
                    impl #implementation_generics ::nota::NotaDecodeTraced for #name #type_generics #where_clause {
                        fn instance_reference() -> ::nota::TypeReference {
                            ::nota::TypeReference::named(#type_name)
                        }

                        fn from_nota_block_traced(
                            block: &::nota::Block,
                        ) -> ::std::result::Result<::nota::DecodedWithSchema<Self>, ::nota::NotaDecodeError> {
                            let children = ::nota::NotaBlock::new(block).expect_children(
                                ::nota::Delimiter::Parenthesis,
                                #type_name,
                                #field_count,
                            )?;
                            let mut nodes: ::std::vec::Vec<::nota::InstanceSchema> =
                                ::std::vec::Vec::with_capacity(#field_count);
                            let value = Self {
                                #(#field_steps,)*
                            };
                            ::std::result::Result::Ok(::nota::DecodedWithSchema::new(
                                value,
                                ::nota::InstanceSchema::new(
                                    <Self as ::nota::NotaDecodeTraced>::instance_reference(),
                                    ::nota::InstanceSchemaBody::Struct(nodes),
                                ),
                            ))
                        }
                    }
                }
            }
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let field_type = &fields.unnamed.first().expect("one field checked").ty;
                quote! {
                    impl #implementation_generics ::nota::NotaDecodeTraced for #name #type_generics #where_clause {
                        fn instance_reference() -> ::nota::TypeReference {
                            ::nota::TypeReference::named(#type_name)
                        }

                        fn from_nota_block_traced(
                            block: &::nota::Block,
                        ) -> ::std::result::Result<::nota::DecodedWithSchema<Self>, ::nota::NotaDecodeError> {
                            let inner = <#field_type as ::nota::NotaDecodeTraced>::from_nota_block_traced(block)?;
                            let (inner_value, inner_schema) = inner.into_parts();
                            ::std::result::Result::Ok(::nota::DecodedWithSchema::new(
                                Self(inner_value),
                                ::nota::InstanceSchema::new(
                                    <Self as ::nota::NotaDecodeTraced>::instance_reference(),
                                    ::nota::InstanceSchemaBody::Newtype(::std::boxed::Box::new(inner_schema)),
                                ),
                            ))
                        }
                    }
                }
            }
            Fields::Unnamed(fields) => Error::new_spanned(
                fields,
                "NotaDecodeTraced supports named structs or one-field tuple newtypes",
            )
            .to_compile_error(),
            Fields::Unit => Error::new_spanned(
                name,
                "NotaDecodeTraced supports named structs or one-field tuple newtypes",
            )
            .to_compile_error(),
        }
    }
}

struct TracedFieldStep<'field> {
    index: usize,
    field: &'field Field,
}

impl<'field> TracedFieldStep<'field> {
    fn new(index: usize, field: &'field Field) -> Self {
        Self { index, field }
    }

    fn body_named(&self) -> TokenStreamTwo {
        let name = self.field.ident.as_ref().expect("named field");
        let field_type = &self.field.ty;
        let index = Index::from(self.index);
        quote! {
            #name: {
                let decoded = <#field_type as ::nota::NotaDecodeTraced>::from_nota_block_traced(&children[#index])?;
                let (field_value, field_schema) = decoded.into_parts();
                nodes.push(field_schema);
                field_value
            }
        }
    }
}

struct EnumTrace {
    name: Ident,
    generics: Generics,
    data: DataEnum,
}

impl EnumTrace {
    fn new(name: Ident, generics: Generics, data: DataEnum) -> Self {
        Self {
            name,
            generics,
            data,
        }
    }

    fn expand(self) -> TokenStreamTwo {
        let name = self.name;
        let enum_name = name.to_string();
        let (implementation_generics, type_generics, where_clause) = self.generics.split_for_impl();
        let unit_arms = self
            .data
            .variants
            .iter()
            .filter(|variant| matches!(variant.fields, Fields::Unit))
            .map(|variant| TracedUnitVariant::new(&name, variant).arm())
            .collect::<Vec<_>>();
        let payload_arms = self
            .data
            .variants
            .iter()
            .filter(|variant| !matches!(variant.fields, Fields::Unit))
            .map(|variant| TracedPayloadVariant::new(&name, variant).arm())
            .collect::<Vec<_>>();
        // When the enum has no unit variants, a bare atom can never be a valid
        // variant, so the atom branch reports an unknown variant directly. This
        // avoids an unreachable `Ok(...)` after an all-diverging match arm.
        let atom_branch = if unit_arms.is_empty() {
            quote! {
                if let Some(variant) = block.demote_to_string() {
                    return ::std::result::Result::Err(::nota::NotaDecodeError::UnknownVariant {
                        enum_name: #enum_name,
                        variant: variant.to_owned(),
                    });
                }
            }
        } else {
            quote! {
                if let Some(variant) = block.demote_to_string() {
                    let value = match variant {
                        #(#unit_arms)*
                        other => return ::std::result::Result::Err(::nota::NotaDecodeError::UnknownVariant {
                            enum_name: #enum_name,
                            variant: other.to_owned(),
                        }),
                    };
                    return ::std::result::Result::Ok(::nota::DecodedWithSchema::new(
                        value,
                        ::nota::InstanceSchema::new(expected, ::nota::InstanceSchemaBody::EnumPayload(::std::option::Option::None)),
                    ));
                }
            }
        };
        quote! {
            impl #implementation_generics ::nota::NotaDecodeTraced for #name #type_generics #where_clause {
                fn instance_reference() -> ::nota::TypeReference {
                    ::nota::TypeReference::named(#enum_name)
                }

                fn from_nota_block_traced(
                    block: &::nota::Block,
                ) -> ::std::result::Result<::nota::DecodedWithSchema<Self>, ::nota::NotaDecodeError> {
                    let expected = <Self as ::nota::NotaDecodeTraced>::instance_reference();
                    #atom_branch
                    let children = ::nota::NotaBlock::new(block).expect_body(
                        ::nota::Delimiter::Parenthesis,
                        #enum_name,
                    )?;
                    let children = children.expect_fields(#enum_name, 2)?;
                    let variant = children[0].demote_to_string().ok_or(::nota::NotaDecodeError::ExpectedAtom {
                        type_name: "enum variant",
                    })?;
                    match variant {
                        #(#payload_arms)*
                        other => ::std::result::Result::Err(::nota::NotaDecodeError::UnknownVariant {
                            enum_name: #enum_name,
                            variant: other.to_owned(),
                        }),
                    }
                }
            }
        }
    }
}

struct TracedUnitVariant<'variant> {
    enum_name: &'variant Ident,
    variant: &'variant Variant,
}

impl<'variant> TracedUnitVariant<'variant> {
    fn new(enum_name: &'variant Ident, variant: &'variant Variant) -> Self {
        Self { enum_name, variant }
    }

    fn arm(&self) -> TokenStreamTwo {
        let enum_name = self.enum_name;
        let variant_name = &self.variant.ident;
        let tag = variant_name.to_string();
        quote! {
            #tag => #enum_name::#variant_name,
        }
    }
}

struct TracedPayloadVariant<'variant> {
    enum_name: &'variant Ident,
    variant: &'variant Variant,
}

impl<'variant> TracedPayloadVariant<'variant> {
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
                    #tag => {
                        let payload = <#field_type as ::nota::NotaDecodeTraced>::from_nota_block_traced(&children[1])?;
                        let (payload_value, payload_schema) = payload.into_parts();
                        ::std::result::Result::Ok(::nota::DecodedWithSchema::new(
                            #enum_name::#variant_name(payload_value),
                            ::nota::InstanceSchema::new(
                                expected,
                                ::nota::InstanceSchemaBody::EnumPayload(::std::option::Option::Some(::std::boxed::Box::new(payload_schema))),
                            ),
                        ))
                    }
                }
            }
            Fields::Unnamed(_) => Error::new_spanned(
                variant_name,
                "NotaDecodeTraced enum payload variants must carry exactly one unnamed field",
            )
            .to_compile_error(),
            Fields::Named(fields) => Error::new_spanned(
                fields,
                "NotaDecodeTraced enum payload variants must carry one unnamed field, not named fields",
            )
            .to_compile_error(),
            Fields::Unit => {
                Error::new_spanned(variant_name, "unit variants are handled by the atom branch")
                    .to_compile_error()
            }
        }
    }
}

struct GenericsWithTracedBound {
    generics: Generics,
}

impl GenericsWithTracedBound {
    fn new(generics: Generics) -> Self {
        Self { generics }
    }

    fn generics(mut self) -> Generics {
        for parameter in self.generics.params.iter_mut() {
            if let GenericParam::Type(type_parameter) = parameter {
                type_parameter
                    .bounds
                    .push(syn::parse_quote!(::nota::NotaDecodeTraced));
            }
        }
        self.generics
    }
}
