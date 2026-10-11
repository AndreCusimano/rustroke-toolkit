//! Rust code → model.
//!
//! The file is parsed with `syn`. The view struct is the one with an
//! `impl Struct { fn show(&mut self, ui) }`; the statements of `show` that
//! match the code the generator writes become nodes, everything else
//! becomes free code ([`WidgetKind::RawCode`]) copied from the source text,
//! comments included. Other fields, methods and items are kept as written.

use proc_macro2::{LineColumn, TokenStream, TokenTree};
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::{
    Block, Expr, ExprMethodCall, Fields, ImplItem, Item, Lit, Pat, RangeLimits, Stmt, Token, Type,
    UnOp, UseTree,
};

use std::collections::BTreeMap;

use crate::builders::{Arg, builder_methods};
use crate::codegen::{HEADER_PREFIX, MANAGED_IMPORTS};
use crate::kind::{CONTENT_STYLE, WidgetKind, content_style_target};
use crate::model::{Node, RawField, View, WindowSettings};
use crate::value::{Font, FontFamily, PropKind, Shadow, Sizing, Track, Value, format_tracks};

/// Names of the layouts rustroke 0.20 deprecated, which the parser turns
/// into stacks and grids.
const LEGACY_IMPORTS: &[&str] = &[
    "Container",
    "Flex",
    "FlexAlign",
    "FlexDirection",
    "FlexGrid",
    "FlexItem",
    "FlexJustify",
    "FlexUi",
    "GridUi",
];

/// Why a file couldn't be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// Line, starting at 1.
    pub line: usize,
    /// Column in characters, starting at 0.
    pub column: usize,
    /// What is wrong.
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column + 1, self.message)
    }
}

impl std::error::Error for ParseError {}

/// Reads the code of view `name` (the file `<name>.rs`).
pub fn parse_view(name: &str, source: &str) -> Result<View, ParseError> {
    let result = parse_inner(name, source);
    // proc-macro2 keeps every parsed source to resolve span locations; free
    // it, since the editor parses after every change. No span outlives
    // `parse_inner`.
    proc_macro2::extra::invalidate_current_thread_spans();
    result
}

fn parse_inner(name: &str, source: &str) -> Result<View, ParseError> {
    let file = syn::parse_file(source).map_err(|err| {
        let start = err.span().start();
        ParseError {
            line: start.line,
            column: start.column,
            message: err.to_string(),
        }
    })?;
    let mut parser = Parser {
        src: source,
        line_starts: std::iter::once(0)
            .chain(source.match_indices('\n').map(|(i, _)| i + 1))
            .collect(),
        view: View::new(name),
        in_plot: false,
    };
    parser.file(&file)?;
    Ok(parser.view)
}

struct Parser<'s> {
    src: &'s str,
    line_starts: Vec<usize>,
    view: View,
    /// Reading the closure of a `Plot`: only series are nodes there.
    in_plot: bool,
}

// ---- Expression helpers ----------------------------------------------------

fn lit_str(e: &Expr) -> Option<String> {
    match e {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Str(s) => Some(s.value()),
            _ => None,
        },
        _ => None,
    }
}

fn lit_bool(e: &Expr) -> Option<bool> {
    match e {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Bool(b) => Some(b.value),
            _ => None,
        },
        _ => None,
    }
}

fn lit_float(e: &Expr) -> Option<f64> {
    match e {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Float(f) if f.suffix().is_empty() || f.suffix() == "f32" => f.base10_parse().ok(),
            Lit::Int(i) if i.suffix().is_empty() => i.base10_parse().ok(),
            _ => None,
        },
        Expr::Unary(unary) if matches!(unary.op, UnOp::Neg(_)) => {
            lit_float(&unary.expr).map(|v| -v)
        }
        _ => None,
    }
}

fn lit_int(e: &Expr) -> Option<i64> {
    match e {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Int(i) if i.suffix().is_empty() || i.suffix() == "usize" => i.base10_parse().ok(),
            _ => None,
        },
        _ => None,
    }
}

fn lit_range(e: &Expr) -> Option<(f64, f64)> {
    match e {
        Expr::Range(range) if matches!(range.limits, RangeLimits::Closed(_)) => Some((
            lit_float(range.start.as_deref()?)?,
            lit_float(range.end.as_deref()?)?,
        )),
        _ => None,
    }
}

fn lit_u8(e: &Expr) -> Option<u8> {
    match e {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Int(i) if i.suffix().is_empty() || i.suffix() == "u8" => i.base10_parse().ok(),
            _ => None,
        },
        _ => None,
    }
}

/// `Color::from_srgb8(r, g, b)`, `Color::from_srgba8(r, g, b, a)`,
/// `Color::WHITE`, `Color::BLACK` or `Color::TRANSPARENT`.
fn lit_color(e: &Expr) -> Option<[u8; 4]> {
    if let Expr::Path(p) = e {
        let segments = &p.path.segments;
        if p.qself.is_some() || segments.len() != 2 || segments[0].ident != "Color" {
            return None;
        }
        return match segments[1].ident.to_string().as_str() {
            "WHITE" => Some([255, 255, 255, 255]),
            "BLACK" => Some([0, 0, 0, 255]),
            "TRANSPARENT" => Some([0, 0, 0, 0]),
            _ => None,
        };
    }
    let chain = chain(e)?;
    if chain.ty != "Color" || !chain.calls.is_empty() {
        return None;
    }
    let args: Vec<u8> = chain.args.iter().map(lit_u8).collect::<Option<_>>()?;
    match (chain.ctor.as_str(), &args[..]) {
        ("from_srgb8", &[r, g, b]) => Some([r, g, b, 255]),
        ("from_srgba8", &[r, g, b, a]) => Some([r, g, b, a]),
        _ => None,
    }
}

/// A float literal or `f32::INFINITY` (no limit).
fn lit_side(e: &Expr) -> Option<f64> {
    match e {
        Expr::Path(p) if compact(p) == "f32::INFINITY" => Some(f64::INFINITY),
        _ => lit_float(e),
    }
}

/// `Shadow { offset: vec2(x, y), blur: b, spread: s, color: c }`, fields in
/// any order.
fn lit_shadow(e: &Expr) -> Option<Shadow> {
    let Expr::Struct(s) = e else {
        return None;
    };
    if s.qself.is_some() || !s.path.is_ident("Shadow") || s.rest.is_some() || s.fields.len() != 4 {
        return None;
    }
    let mut shadow = Shadow::default();
    let mut seen = Vec::new();
    for field in &s.fields {
        let syn::Member::Named(name) = &field.member else {
            return None;
        };
        let name = name.to_string();
        match name.as_str() {
            "offset" => shadow.offset = lit_size(&field.expr)?,
            "blur" => shadow.blur = lit_float(&field.expr)?,
            "spread" => shadow.spread = lit_float(&field.expr)?,
            "color" => shadow.color = lit_color(&field.expr)?,
            _ => return None,
        }
        if seen.contains(&name) {
            return None;
        }
        seen.push(name);
    }
    Some(shadow)
}

/// `vec2(x, y)` or `Vec2::new(x, y)`; a side may be `f32::INFINITY`.
fn lit_size(e: &Expr) -> Option<(f64, f64)> {
    let Expr::Call(call) = e else {
        return None;
    };
    let Expr::Path(path) = &*call.func else {
        return None;
    };
    let name: Vec<String> = path
        .path
        .segments
        .iter()
        .map(|s| s.ident.to_string())
        .collect();
    if path.qself.is_some() || !(name == ["vec2"] || name == ["Vec2", "new"]) {
        return None;
    }
    match call.args.iter().collect::<Vec<_>>()[..] {
        [x, y] => Some((lit_side(x)?, lit_side(y)?)),
        _ => None,
    }
}

/// `Stroke::new(width, color)`
fn lit_stroke(e: &Expr) -> Option<(f64, [u8; 4])> {
    let chain = chain(e)?;
    if chain.ty != "Stroke" || chain.ctor != "new" || !chain.calls.is_empty() {
        return None;
    }
    match chain.args.iter().collect::<Vec<_>>()[..] {
        [width, color] => Some((lit_float(width)?, lit_color(color)?)),
        _ => None,
    }
}

/// `TextStyle::proportional(size)` (or `monospace`, `system`), then
/// `.weight(w)` and `.bold()`.
fn lit_font(e: &Expr) -> Option<Font> {
    let chain = chain(e)?;
    let family = FontFamily::from_ctor(&chain.ctor)?;
    let [size] = chain.args.iter().collect::<Vec<_>>()[..] else {
        return None;
    };
    if chain.ty != "TextStyle" {
        return None;
    }
    let mut font = Font {
        family,
        size: lit_float(size)?,
        ..Font::default()
    };
    for call in &chain.calls {
        match (call.method.to_string().as_str(), call.args.len()) {
            ("weight", 1) => font.weight = lit_u16(&call.args[0])?,
            ("bold", 0) => font.bold = true,
            _ => return None,
        }
    }
    Some(font)
}

fn lit_u16(e: &Expr) -> Option<u16> {
    match e {
        Expr::Lit(lit) => match &lit.lit {
            Lit::Int(i) if i.suffix().is_empty() || i.suffix() == "u16" => i.base10_parse().ok(),
            _ => None,
        },
        _ => None,
    }
}

/// `vec![Track::Points(120.0), Track::Fraction(1.0), Track::Auto]`
fn lit_tracks(e: &Expr) -> Option<Vec<Track>> {
    let Expr::Macro(mac) = e else {
        return None;
    };
    if !mac.mac.path.is_ident("vec") {
        return None;
    }
    let items = mac
        .mac
        .parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)
        .ok()?;
    items
        .iter()
        .map(|item| match item {
            Expr::Path(p) if p.path.segments.len() == 2 => (p.path.segments[0].ident == "Track"
                && p.path.segments[1].ident == "Auto")
                .then_some(Track::Auto),
            Expr::Call(call) => {
                let Expr::Path(p) = &*call.func else {
                    return None;
                };
                let segments = &p.path.segments;
                if segments.len() != 2 || segments[0].ident != "Track" || call.args.len() != 1 {
                    return None;
                }
                let v = lit_float(&call.args[0])?;
                match segments[1].ident.to_string().as_str() {
                    "Points" => Some(Track::Points(v)),
                    "Fraction" => Some(Track::Fraction(v)),
                    _ => None,
                }
            }
            _ => None,
        })
        .collect()
}

/// `Sizing::Content`, `Sizing::Fill` or `Sizing::Points(n)`
fn lit_sizing(e: &Expr) -> Option<Sizing> {
    match e {
        Expr::Path(p) => match compact(p).as_str() {
            "Sizing::Content" => Some(Sizing::Content),
            "Sizing::Fill" => Some(Sizing::Fill),
            _ => None,
        },
        Expr::Call(call) if compact(&call.func) == "Sizing::Points" && call.args.len() == 1 => {
            Some(Sizing::Points(lit_float(&call.args[0])?))
        }
        _ => None,
    }
}

/// `Axis::new().label("title")`
fn lit_axis_label(e: &Expr) -> Option<String> {
    let chain = chain(e)?;
    match (chain.ty.as_str(), chain.ctor.as_str(), &chain.calls[..]) {
        ("Axis", "new", [label]) if chain.args.is_empty() && label.method == "label" => {
            match label.args.iter().collect::<Vec<_>>()[..] {
                [text] => lit_str(text),
                _ => None,
            }
        }
        _ => None,
    }
}

/// `LineStyle::Solid`, `LineStyle::dashed()` or `LineStyle::dotted()`
fn lit_line_style(e: &Expr) -> Option<&'static str> {
    if let Expr::Path(p) = e {
        return (compact(p) == "LineStyle::Solid").then_some("solid");
    }
    let chain = chain(e)?;
    if chain.ty != "LineStyle" || !chain.args.is_empty() || !chain.calls.is_empty() {
        return None;
    }
    match chain.ctor.as_str() {
        "dashed" => Some("dashed"),
        "dotted" => Some("dotted"),
        _ => None,
    }
}

/// `Type::Variant` → `Variant`
fn variant<'e>(e: &'e Expr, ty: &str) -> Option<&'e syn::Ident> {
    let Expr::Path(p) = e else {
        return None;
    };
    let segments = &p.path.segments;
    (p.qself.is_none() && segments.len() == 2 && segments[0].ident == ty)
        .then(|| &segments[1].ident)
}

/// `&self.name`
fn ref_self_field(e: &Expr) -> Option<String> {
    match e {
        Expr::Reference(r) if r.mutability.is_none() => self_field(&r.expr),
        _ => None,
    }
}

/// `self.name.clone()`
fn cloned_self_field(e: &Expr) -> Option<String> {
    match e {
        Expr::MethodCall(call) if call.method == "clone" && call.args.is_empty() => {
            self_field(&call.receiver)
        }
        _ => None,
    }
}

/// The code of `tokens` without spaces, to compare with a known shape.
fn compact(tokens: impl ToTokens) -> String {
    tokens.to_token_stream().to_string().replace(' ', "")
}

/// `ui.style_mut().visuals.text = …;`: a content style property.
fn style_stmt(stmt: &Stmt, ui: &str) -> Option<(&'static str, Value)> {
    let Stmt::Expr(Expr::Assign(assign), Some(_)) = stmt else {
        return None;
    };
    if !assign.attrs.is_empty() {
        return None;
    }
    let target = compact(&assign.left);
    let spec = CONTENT_STYLE.iter().find(|spec| {
        content_style_target(spec.key).is_some_and(|t| target == format!("{ui}.style_mut().{t}"))
    })?;
    let value = match spec.kind {
        PropKind::Color => Value::Color(lit_color(&assign.right)?),
        PropKind::Size => {
            let (w, h) = lit_size(&assign.right)?;
            Value::Size(w, h)
        }
        _ => return None,
    };
    spec.kind.accepts(&value).then_some((spec.key, value))
}

/// `self.name`
fn self_field(e: &Expr) -> Option<String> {
    match e {
        Expr::Field(field) => {
            let syn::Member::Named(name) = &field.member else {
                return None;
            };
            matches!(&*field.base, Expr::Path(p) if p.path.is_ident("self"))
                .then(|| name.to_string())
        }
        _ => None,
    }
}

/// `&mut self.name`
fn mut_self_field(e: &Expr) -> Option<String> {
    match e {
        Expr::Reference(r) if r.mutability.is_some() => self_field(&r.expr),
        _ => None,
    }
}

/// `&mut self.name.<index>`
fn mut_self_field_index(e: &Expr, index: u32) -> Option<String> {
    let Expr::Reference(r) = e else {
        return None;
    };
    let Expr::Field(field) = &*r.expr else {
        return None;
    };
    match &field.member {
        syn::Member::Unnamed(i) if r.mutability.is_some() && i.index == index => {
            self_field(&field.base)
        }
        _ => None,
    }
}

/// The first `self.name` anywhere in `e`.
fn find_self_field(e: &Expr) -> Option<String> {
    fn walk(tokens: TokenStream) -> Option<String> {
        let tokens: Vec<TokenTree> = tokens.into_iter().collect();
        for (i, tt) in tokens.iter().enumerate() {
            match tt {
                TokenTree::Ident(id) if id == "self" => {
                    if let (Some(TokenTree::Punct(dot)), Some(TokenTree::Ident(name))) =
                        (tokens.get(i + 1), tokens.get(i + 2))
                        && dot.as_char() == '.'
                    {
                        return Some(name.to_string());
                    }
                }
                TokenTree::Group(g) => {
                    if let Some(found) = walk(g.stream()) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }
    walk(e.to_token_stream())
}

fn is_ident(e: &Expr, name: &str) -> bool {
    matches!(e, Expr::Path(p) if p.qself.is_none() && p.path.is_ident(name))
}

/// `|param| { … }` → the parameter name and the block.
fn closure(e: &Expr) -> Option<(String, &Block)> {
    let Expr::Closure(c) = e else {
        return None;
    };
    if c.inputs.len() != 1 || c.asyncness.is_some() || c.capture.is_some() {
        return None;
    }
    let Pat::Ident(param) = &c.inputs[0] else {
        return None;
    };
    if param.by_ref.is_some() || param.mutability.is_some() || param.subpat.is_some() {
        return None;
    }
    let Expr::Block(body) = &*c.body else {
        return None;
    };
    (body.label.is_none()).then(|| (param.ident.to_string(), &body.block))
}

/// A builder chain `Type::ctor(args).a(..).b(..)`.
struct Chain<'e> {
    ty: String,
    ctor: String,
    args: &'e Punctuated<Expr, Token![,]>,
    calls: Vec<&'e ExprMethodCall>,
}

fn chain(e: &Expr) -> Option<Chain<'_>> {
    let mut calls = Vec::new();
    let mut e = e;
    while let Expr::MethodCall(call) = e {
        if call.turbofish.is_some() {
            return None;
        }
        calls.push(call);
        e = &call.receiver;
    }
    calls.reverse();
    let Expr::Call(call) = e else {
        return None;
    };
    let Expr::Path(path) = &*call.func else {
        return None;
    };
    let segments = &path.path.segments;
    if path.qself.is_some()
        || path.path.leading_colon.is_some()
        || segments.len() != 2
        || segments.iter().any(|s| !s.arguments.is_none())
    {
        return None;
    }
    Some(Chain {
        ty: segments[0].ident.to_string(),
        ctor: segments[1].ident.to_string(),
        args: &call.args,
        calls,
    })
}

/// Removes the common indentation of the non-blank lines and the blank
/// lines at both ends.
fn dedent(text: &str) -> String {
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    let indent = lines
        .iter()
        .filter(|l| !l.is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    let lines: Vec<&str> = lines
        .iter()
        .map(|l| if l.is_empty() { "" } else { &l[indent..] })
        .collect();
    let first = lines
        .iter()
        .position(|l| !l.is_empty())
        .unwrap_or(lines.len());
    let last = lines
        .iter()
        .rposition(|l| !l.is_empty())
        .map_or(first, |i| i + 1);
    lines[first..last].join("\n")
}

impl Parser<'_> {
    // ---- Source positions ---------------------------------------------------

    fn offset(&self, lc: LineColumn) -> usize {
        let start = self
            .line_starts
            .get(lc.line.saturating_sub(1))
            .copied()
            .unwrap_or(self.src.len());
        let line = &self.src[start..];
        start
            + line
                .char_indices()
                .nth(lc.column)
                .map_or(line.len(), |(i, _)| i)
    }

    /// Byte range covered by `tokens`.
    fn range(&self, tokens: impl ToTokens) -> (usize, usize) {
        let tokens: Vec<TokenTree> = tokens.into_token_stream().into_iter().collect();
        let start = tokens.first().map_or(0, |t| self.offset(t.span().start()));
        let end = tokens.last().map_or(0, |t| self.offset(t.span().end()));
        (start, end)
    }

    fn column(&self, offset: usize) -> usize {
        let line_start = self.src[..offset].rfind('\n').map_or(0, |i| i + 1);
        self.src[line_start..offset].chars().count()
    }

    /// The source between two offsets, de-indented (the first line counts
    /// with the column it starts at).
    fn text(&self, start: usize, end: usize) -> String {
        let padded = " ".repeat(self.column(start)) + &self.src[start..end];
        dedent(&padded)
    }

    /// The comments between two offsets (only whitespace and comments can be
    /// there), with their start offset; `None` if there are none. Generated
    /// header lines (any version) are skipped.
    fn comment(&self, start: usize, end: usize) -> Option<(usize, String)> {
        let gap = &self.src[start..end];
        let first = gap.find(|c: char| !c.is_whitespace())?;
        let text = self.text(start + first, end);
        let text: Vec<&str> = text
            .lines()
            .filter(|l| !l.trim().starts_with(HEADER_PREFIX))
            .collect();
        let text = dedent(&text.join("\n"));
        (!text.is_empty()).then_some((start + first, text))
    }

    fn raw(&mut self, code: String) -> Node {
        let mut node = Node::new(self.view.alloc_id(), WidgetKind::RawCode);
        node.props.insert("code", Value::Text(code));
        node
    }

    fn node(&mut self, kind: WidgetKind) -> Node {
        Node::new(self.view.alloc_id(), kind)
    }

    // ---- File ----------------------------------------------------------------

    fn file(&mut self, file: &syn::File) -> Result<(), ParseError> {
        let expected = self.view.struct_name();
        // The view struct is the one with `fn show` in an inherent impl.
        let show_impl = file.items.iter().position(|item| {
            matches!(item, Item::Impl(imp) if imp.trait_.is_none()
                && imp.items.iter().any(|i| matches!(i, ImplItem::Fn(f) if f.sig.ident == "show")))
        });
        let struct_name = match show_impl.map(|i| &file.items[i]) {
            Some(Item::Impl(imp)) => match &*imp.self_ty {
                Type::Path(p) if p.qself.is_none() => p.path.get_ident().map(|i| i.to_string()),
                _ => None,
            },
            _ => None,
        };
        let Some(struct_name) = struct_name else {
            return Err(ParseError {
                line: 1,
                column: 0,
                message: format!(
                    "missing `impl {expected} {{ pub fn show(&mut self, ui: &mut Ui<'_>) {{ … }} }}`"
                ),
            });
        };
        let struct_index = file
            .items
            .iter()
            .position(|item| matches!(item, Item::Struct(s) if s.ident == struct_name))
            .ok_or_else(|| ParseError {
                line: 1,
                column: 0,
                message: format!("missing `pub struct {struct_name}`"),
            })?;

        let mut pos = 0;
        let mut fields = Vec::new();
        for (index, item) in file.items.iter().enumerate() {
            let (start, end) = self.range(item);
            let comment = self.comment(pos, start);
            pos = end;
            if index == struct_index {
                let Item::Struct(s) = item else {
                    unreachable!()
                };
                self.view.derives = derives(&s.attrs);
                if let Fields::Named(named) = &s.fields {
                    for field in &named.named {
                        let (start, end) = self.range(field);
                        let name = field
                            .ident
                            .as_ref()
                            .map(|i| i.to_string())
                            .unwrap_or_default();
                        fields.push(RawField {
                            name,
                            code: self.text(start, end),
                        });
                    }
                }
            } else if Some(index) == show_impl {
                let Item::Impl(imp) = item else {
                    unreachable!()
                };
                self.show_impl(imp)?;
            } else if let Some(names) = rustroke_import(item) {
                for name in names {
                    if !MANAGED_IMPORTS.contains(&name.as_str())
                        && !self.view.extra_imports.contains(&name)
                    {
                        self.view.extra_imports.push(name);
                    }
                }
            } else {
                let start = comment.map_or(start, |(at, _)| at);
                self.view.extra_items.push(self.text(start, end));
            }
        }

        // Fields bound to widgets are generated; the others are kept.
        let mut bound = Vec::new();
        self.view.walk(|node| {
            if let Some((name, _)) = node.binding() {
                bound.push(name.to_owned());
            }
        });
        self.view.extra_fields = fields
            .into_iter()
            .filter(|field| !bound.contains(&field.name))
            .collect();

        // Layouts of rustroke 0.19 read as stacks and grids: their imports
        // go, unless free code still uses them.
        let mut free = Vec::new();
        self.view.walk(|node| {
            if node.kind == WidgetKind::RawCode {
                free.push(node.text("code").to_owned());
            }
        });
        free.extend(self.view.extra_methods.iter().cloned());
        free.extend(self.view.extra_items.iter().cloned());
        free.extend(self.view.extra_fields.iter().map(|f| f.code.clone()));
        self.view.extra_imports.retain(|name| {
            !LEGACY_IMPORTS.contains(&name.as_str())
                || free.iter().any(|code| crate::codegen::mentions(code, name))
        });
        Ok(())
    }

    fn show_impl(&mut self, imp: &syn::ItemImpl) -> Result<(), ParseError> {
        let mut pos = self.offset(imp.brace_token.span.open().end());
        for item in &imp.items {
            let (start, end) = self.range(item);
            let comment = self.comment(pos, start);
            pos = end;
            match item {
                ImplItem::Fn(f) if f.sig.ident == "show" => {
                    let ui = match f.sig.inputs.iter().nth(1) {
                        Some(syn::FnArg::Typed(arg)) => match &*arg.pat {
                            Pat::Ident(p) => p.ident.to_string(),
                            _ => "ui".to_owned(),
                        },
                        _ => "ui".to_owned(),
                    };
                    let mut style = BTreeMap::new();
                    self.view.nodes = self.block(&f.block, &ui, Some(&mut style));
                    self.view.style = style;
                }
                // Generated from the window settings.
                ImplItem::Fn(f)
                    if f.sig.ident == "window_options" || f.sig.ident == "show_window" => {}
                ImplItem::Const(c)
                    if ["TITLE", "SIZE", "RESIZABLE", "MIN_SIZE", "MAX_SIZE"]
                        .iter()
                        .any(|n| c.ident == n) =>
                {
                    self.window_const(c);
                }
                _ => {
                    let start = comment.map_or(start, |(at, _)| at);
                    self.view.extra_methods.push(self.text(start, end));
                }
            }
        }
        Ok(())
    }

    /// `TITLE`, `SIZE`, `RESIZABLE`, `MIN_SIZE` or `MAX_SIZE`; a value that
    /// isn't a plain literal (or a size out of bounds) keeps the default.
    fn window_const(&mut self, c: &syn::ImplItemConst) {
        let window = &mut self.view.window;
        match c.ident.to_string().as_str() {
            "TITLE" => {
                if let Some(title) = lit_str(&c.expr) {
                    window.title = title;
                }
            }
            "SIZE" => {
                if let Expr::Tuple(t) = &c.expr
                    && let [w, h] = t.elems.iter().collect::<Vec<_>>()[..]
                    && let (Some(w), Some(h)) = (lit_float(w), lit_float(h))
                {
                    let size = WindowSettings {
                        width: w as f32,
                        height: h as f32,
                        ..window.clone()
                    };
                    if size.is_valid() {
                        *window = size;
                    }
                }
            }
            "RESIZABLE" => {
                if let Some(resizable) = lit_bool(&c.expr) {
                    window.resizable = resizable;
                }
            }
            name => {
                // `None` or `Some((w, h))`
                let limit = match &c.expr {
                    e if is_ident(e, "None") => Some(None),
                    Expr::Call(call) if is_ident(&call.func, "Some") && call.args.len() == 1 => {
                        match &call.args[0] {
                            Expr::Tuple(t) => match t.elems.iter().collect::<Vec<_>>()[..] {
                                [w, h] => lit_float(w)
                                    .zip(lit_float(h))
                                    .map(|(w, h)| Some((w as f32, h as f32))),
                                _ => None,
                            },
                            _ => None,
                        }
                    }
                    _ => None,
                };
                let Some(limit) = limit else {
                    return;
                };
                let mut changed = window.clone();
                if name == "MIN_SIZE" {
                    changed.min_size = limit;
                } else {
                    changed.max_size = limit;
                }
                if changed.is_valid() {
                    *window = changed;
                }
            }
        }
    }

    // ---- Statements ------------------------------------------------------------

    /// The nodes of a block. Unrecognized statements become free code;
    /// consecutive ones (and the comments around them) form one node.
    /// With `style`, the block's first statements that set the content
    /// style (`ui.style_mut().… = …;`, without comments) go there.
    fn block(
        &mut self,
        block: &Block,
        ui: &str,
        style: Option<&mut BTreeMap<&'static str, Value>>,
    ) -> Vec<Node> {
        let open = self.offset(block.brace_token.span.open().end());
        let close = self.offset(block.brace_token.span.close().start());
        let mut nodes: Vec<Node> = Vec::new();
        let mut pos = open;
        let mut skip = 0;
        if let Some(style) = style {
            for stmt in &block.stmts {
                let (start, end) = self.range(stmt);
                if self.comment(pos, start).is_some() {
                    break;
                }
                match style_stmt(stmt, ui) {
                    Some((key, value)) if !style.contains_key(key) => {
                        style.insert(key, value);
                        pos = end;
                        skip += 1;
                    }
                    _ => break,
                }
            }
        }
        // Start offset of the free code node being extended, if the last
        // node is one.
        let mut run: Option<usize> = None;
        for stmt in block.stmts.iter().skip(skip) {
            let (start, end) = self.range(stmt);
            let comment = self.comment(pos, start);
            pos = end;
            match self.stmt(stmt, ui) {
                Some(node) => {
                    run = None;
                    if let Some((_, text)) = comment {
                        nodes.push(self.raw(text));
                    }
                    nodes.push(node);
                }
                None => match run {
                    Some(run_start) => {
                        let code = self.text(run_start, end);
                        let last = nodes.last_mut().expect("a run has a node");
                        last.props.insert("code", Value::Text(code));
                    }
                    None => {
                        let run_start = comment.map_or(start, |(at, _)| at);
                        run = Some(run_start);
                        let code = self.text(run_start, end);
                        nodes.push(self.raw(code));
                    }
                },
            }
        }
        if let Some((at, text)) = self.comment(pos, close) {
            match run {
                Some(run_start) => {
                    let code = self.src[at..close].trim_end().len() + at;
                    let code = self.text(run_start, code);
                    let last = nodes.last_mut().expect("a run has a node");
                    last.props.insert("code", Value::Text(code));
                }
                None => nodes.push(self.raw(text)),
            }
        }
        nodes
    }

    fn stmt(&mut self, stmt: &Stmt, ui: &str) -> Option<Node> {
        if self.in_plot {
            return match stmt {
                Stmt::Expr(Expr::MethodCall(call), Some(_)) if is_ident(&call.receiver, ui) => {
                    self.series(call)
                }
                _ => None,
            };
        }
        match stmt {
            Stmt::Expr(Expr::If(e), None) => {
                self.button_if(e, ui).or_else(|| self.choice_if(e, ui))
            }
            Stmt::Expr(e, Some(_)) => self.expr(e, ui),
            _ => None,
        }
    }

    /// `if ui.button(text).clicked() { … }`, or with `ui.add(Button…)` /
    /// `ui.add_sized(size, Button…)`.
    fn button_if(&mut self, e: &syn::ExprIf, ui: &str) -> Option<Node> {
        if e.else_branch.is_some() || !e.attrs.is_empty() {
            return None;
        }
        let Expr::MethodCall(clicked) = &*e.cond else {
            return None;
        };
        if clicked.method != "clicked" || !clicked.args.is_empty() {
            return None;
        }
        let Expr::MethodCall(button) = &*clicked.receiver else {
            return None;
        };
        if !is_ident(&button.receiver, ui) || button.turbofish.is_some() {
            return None;
        }
        let mut node = self.ui_call(button, ui)?;
        if node.kind != WidgetKind::Button {
            return None;
        }
        let open = self.offset(e.then_branch.brace_token.span.open().end());
        let close = self.offset(e.then_branch.brace_token.span.close().start());
        node.props
            .insert("on_click", Value::Text(self.text(open, close)));
        Some(node)
    }

    /// `if ui.add(RadioButton::new(self.b == v, text)…).clicked() { self.b = v; }`
    /// (or `SelectableLabel`): a styled radio button or selectable label.
    fn choice_if(&mut self, e: &syn::ExprIf, ui: &str) -> Option<Node> {
        if e.else_branch.is_some() || !e.attrs.is_empty() {
            return None;
        }
        let Expr::MethodCall(clicked) = &*e.cond else {
            return None;
        };
        if clicked.method != "clicked" || !clicked.args.is_empty() {
            return None;
        }
        let Expr::MethodCall(add) = &*clicked.receiver else {
            return None;
        };
        if !is_ident(&add.receiver, ui) || add.method != "add" || add.args.len() != 1 {
            return None;
        }
        let chain = chain(&add.args[0])?;
        let kind = match (chain.ty.as_str(), chain.ctor.as_str()) {
            ("RadioButton", "new") => WidgetKind::RadioValue,
            ("SelectableLabel", "new") => WidgetKind::SelectableValue,
            _ => return None,
        };
        let [selected, text] = chain.args.iter().collect::<Vec<_>>()[..] else {
            return None;
        };
        let Expr::Binary(eq) = selected else {
            return None;
        };
        if !matches!(eq.op, syn::BinOp::Eq(_)) {
            return None;
        }
        let binding = self_field(&eq.left)?;
        let value = lit_int(&eq.right)?;
        // The body: `self.b = v;`
        let [Stmt::Expr(Expr::Assign(assign), Some(_))] = &e.then_branch.stmts[..] else {
            return None;
        };
        if self_field(&assign.left)? != binding || lit_int(&assign.right)? != value {
            return None;
        }
        let mut node = self.node(kind);
        node.props.insert("binding", Value::Text(binding));
        node.props.insert("value", Value::Int(value));
        node.props.insert("text", Value::Text(lit_str(text)?));
        apply_builders(&mut node, &chain.calls)?;
        Some(node)
    }

    fn expr(&mut self, e: &Expr, ui: &str) -> Option<Node> {
        if let Expr::MethodCall(call) = e
            && is_ident(&call.receiver, ui)
            && call.turbofish.is_none()
        {
            return self.ui_call(call, ui);
        }
        // `Type::new(..).builder(..).show(ui, |ui| …)`
        let chain = chain(e)?;
        let (last, builders) = chain.calls.split_last()?;
        let terminal = last.method.to_string();
        if (chain.ty.as_str(), chain.ctor.as_str(), terminal.as_str())
            == ("PieChart", "new", "show")
        {
            // `PieChart::new(self.x.clone())….show(ui)`
            let ([slices], [arg]) = (
                &chain.args.iter().collect::<Vec<_>>()[..],
                &last.args.iter().collect::<Vec<_>>()[..],
            ) else {
                return None;
            };
            if !is_ident(arg, ui) {
                return None;
            }
            let mut node = self.node(WidgetKind::PieChart);
            node.props
                .insert("binding", Value::Text(cloned_self_field(slices)?));
            apply_builders(&mut node, builders)?;
            return Some(node);
        }
        if let Some(node) = self.legacy(&chain, ui) {
            return Some(node);
        }
        let kind = match (chain.ty.as_str(), chain.ctor.as_str(), terminal.as_str()) {
            ("Grid", "new", "show") => WidgetKind::Grid,
            ("CollapsingHeader", "new", "show") => WidgetKind::Collapsing,
            ("ScrollArea", "vertical" | "horizontal" | "both", "show") => WidgetKind::ScrollArea,
            ("Stack", "row" | "column" | "overlay", "show") => WidgetKind::Stack,
            ("Plot", "new", "show") => WidgetKind::Plot,
            ("Panel", "left" | "right" | "top" | "bottom", "show_inside") => WidgetKind::Panel,
            ("CentralPanel", "default", "show_inside") => WidgetKind::CentralPanel,
            ("ComboBox", "from_label", "show_ui") => WidgetKind::ComboBox,
            _ => return None,
        };
        if last.args.len() != 2 || !is_ident(&last.args[0], ui) {
            return None;
        }
        let (param, body) = closure(&last.args[1])?;
        let mut node = self.node(kind);
        match kind {
            WidgetKind::Grid | WidgetKind::Plot => {
                let [id] = chain.args.iter().collect::<Vec<_>>()[..] else {
                    return None;
                };
                node.props.insert("id", Value::Text(lit_str(id)?));
            }
            WidgetKind::Stack => {
                if !chain.args.is_empty() {
                    return None;
                }
                node.props
                    .insert("direction", Value::Text(chain.ctor.clone()));
            }
            WidgetKind::Collapsing => {
                let [text] = chain.args.iter().collect::<Vec<_>>()[..] else {
                    return None;
                };
                node.props.insert("text", Value::Text(lit_str(text)?));
            }
            WidgetKind::ScrollArea => {
                if !chain.args.is_empty() {
                    return None;
                }
                node.props
                    .insert("direction", Value::Text(chain.ctor.clone()));
            }
            WidgetKind::CentralPanel => {
                if !chain.args.is_empty() {
                    return None;
                }
            }
            WidgetKind::Panel => {
                let [id] = chain.args.iter().collect::<Vec<_>>()[..] else {
                    return None;
                };
                node.props.insert("id", Value::Text(lit_str(id)?));
                node.props.insert("side", Value::Text(chain.ctor.clone()));
            }
            WidgetKind::ComboBox => {
                let [label] = chain.args.iter().collect::<Vec<_>>()[..] else {
                    return None;
                };
                node.props.insert("label", Value::Text(lit_str(label)?));
            }
            _ => unreachable!(),
        }
        apply_builders(&mut node, builders)?;
        match kind {
            WidgetKind::ComboBox => {
                let selected = builders
                    .iter()
                    .find(|c| c.method == "selected_text")
                    .and_then(|c| c.args.first())
                    .and_then(find_self_field);
                let (binding, items) = combo_items(body, &param, selected)?;
                node.props.insert("binding", Value::Text(binding));
                node.props.insert("items", Value::Items(items));
            }
            WidgetKind::Plot => {
                self.in_plot = true;
                node.children = self.block(body, &param, None);
                self.in_plot = false;
            }
            _ => node.children = self.block(body, &param, Some(&mut node.props)),
        }
        Some(node)
    }

    /// `plot.line(Line::new(&self.x)…)`, `plot.points(Points::new(&self.x)…)`,
    /// `plot.hline(HLine::new(y)…)` or `plot.vline(VLine::new(x)…)`.
    fn series(&mut self, call: &ExprMethodCall) -> Option<Node> {
        let [arg] = call.args.iter().collect::<Vec<_>>()[..] else {
            return None;
        };
        let chain = chain(arg)?;
        let kind = match (call.method.to_string().as_str(), chain.ty.as_str()) {
            ("line", "Line") => WidgetKind::PlotLine,
            ("points", "Points") => WidgetKind::PlotPoints,
            ("hline", "HLine") => WidgetKind::PlotHLine,
            ("vline", "VLine") => WidgetKind::PlotVLine,
            _ => return None,
        };
        let [value] = chain.args.iter().collect::<Vec<_>>()[..] else {
            return None;
        };
        if chain.ctor != "new" || call.turbofish.is_some() {
            return None;
        }
        let mut node = self.node(kind);
        match kind {
            WidgetKind::PlotLine | WidgetKind::PlotPoints => {
                node.props
                    .insert("binding", Value::Text(ref_self_field(value)?));
            }
            _ => {
                node.props.insert("value", Value::Float(lit_float(value)?));
            }
        }
        apply_builders(&mut node, &chain.calls)?;
        Some(node)
    }

    /// Code written for rustroke 0.19 (toolkit 1.x) with the layouts it
    /// deprecated: `Container`, `Flex` and `FlexGrid` become a stack or a
    /// grid. `None` for anything else (or a shape it doesn't know).
    fn legacy(&mut self, chain: &Chain<'_>, ui: &str) -> Option<Node> {
        let (last, builders) = chain.calls.split_last()?;
        if last.method != "show" || last.args.len() != 2 || !is_ident(&last.args[0], ui) {
            return None;
        }
        let args: Vec<&Expr> = chain.args.iter().collect();
        let (param, body) = closure(&last.args[1])?;
        let mut node = match (chain.ty.as_str(), chain.ctor.as_str(), &args[..]) {
            ("Container", "new", []) => {
                let mut node = self.node(WidgetKind::Stack);
                node.props.insert("direction", Value::Text("column".into()));
                node
            }
            ("Flex", "row" | "column", [_id]) => {
                let mut node = self.node(WidgetKind::Stack);
                node.props
                    .insert("direction", Value::Text(chain.ctor.clone()));
                node
            }
            ("FlexGrid", "new", [id, columns]) => {
                let mut node = self.node(WidgetKind::Grid);
                node.props.insert("id", Value::Text(lit_str(id)?));
                let columns = format_tracks(&lit_tracks(columns)?);
                node.props.insert("columns", Value::Text(columns));
                node
            }
            _ => return None,
        };
        legacy_builders(&mut node, builders)?;
        match chain.ty.as_str() {
            "Container" => node.children = self.block(body, &param, Some(&mut node.props)),
            _ => {
                // `flex.add(FlexItem…, |ui| …);` / `grid.add(GridCell…, |ui| …);`
                for stmt in &body.stmts {
                    let Stmt::Expr(Expr::MethodCall(call), Some(_)) = stmt else {
                        return None;
                    };
                    if !is_ident(&call.receiver, &param)
                        || call.method != "add"
                        || call.args.len() != 2
                    {
                        return None;
                    }
                    let item = self::chain(&call.args[0])?;
                    let (param, block) = closure(&call.args[1])?;
                    let mut child = match (item.ty.as_str(), item.ctor.as_str()) {
                        ("FlexItem", "new") if node.kind == WidgetKind::Stack => {
                            let mut child = self.node(WidgetKind::StackItem);
                            legacy_builders(&mut child, &item.calls)?;
                            child
                        }
                        ("GridCell", "at") if node.kind == WidgetKind::Grid => {
                            let mut child = self.node(WidgetKind::GridCell);
                            let [column, row] = item.args.iter().collect::<Vec<_>>()[..] else {
                                return None;
                            };
                            child.props.insert("column", Value::Int(lit_int(column)?));
                            child.props.insert("row", Value::Int(lit_int(row)?));
                            match item.calls[..] {
                                [] => {}
                                [span] if span.method == "span" && span.args.len() == 2 => {
                                    child
                                        .props
                                        .insert("column_span", Value::Int(lit_int(&span.args[0])?));
                                    child
                                        .props
                                        .insert("row_span", Value::Int(lit_int(&span.args[1])?));
                                }
                                _ => return None,
                            }
                            child
                        }
                        _ => return None,
                    };
                    child.children = self.block(block, &param, Some(&mut child.props));
                    node.children.push(child);
                }
            }
        }
        let valid = node.props.iter().all(|(key, value)| {
            node.kind
                .prop(key)
                .is_some_and(|spec| spec.kind.accepts(value))
        });
        valid.then_some(node)
    }

    /// `ui.method(..)`
    fn ui_call(&mut self, call: &ExprMethodCall, ui: &str) -> Option<Node> {
        let args: Vec<&Expr> = call.args.iter().collect();
        let method = call.method.to_string();
        let text = |node: &mut Node, key: &'static str, e: &Expr| -> Option<()> {
            node.props.insert(key, Value::Text(lit_str(e)?));
            Some(())
        };
        let binding = |node: &mut Node, e: &Expr| -> Option<()> {
            node.props
                .insert("binding", Value::Text(mut_self_field(e)?));
            Some(())
        };
        let container = |this: &mut Self, kind: WidgetKind, e: &Expr| -> Option<Node> {
            let (param, block) = closure(e)?;
            let mut node = this.node(kind);
            node.children = this.block(block, &param, Some(&mut node.props));
            Some(node)
        };
        let node = match (method.as_str(), &args[..]) {
            ("label", [t]) => {
                let mut node = self.node(WidgetKind::Label);
                text(&mut node, "text", t)?;
                node
            }
            ("heading", [t]) => {
                let mut node = self.node(WidgetKind::Heading);
                text(&mut node, "text", t)?;
                node
            }
            ("hyperlink", [url]) => {
                let mut node = self.node(WidgetKind::Hyperlink);
                node.props.insert("text", Value::Text(String::new()));
                text(&mut node, "url", url)?;
                node
            }
            ("hyperlink_to", [t, url]) => {
                let mut node = self.node(WidgetKind::Hyperlink);
                text(&mut node, "text", t)?;
                text(&mut node, "url", url)?;
                node
            }
            ("separator", []) => self.node(WidgetKind::Separator),
            ("end_row", []) => self.node(WidgetKind::EndRow),
            ("spinner", []) => self.node(WidgetKind::Spinner),
            ("add_space", [amount]) => {
                let mut node = self.node(WidgetKind::Space);
                node.props
                    .insert("amount", Value::Float(lit_float(amount)?));
                node
            }
            ("button", [t]) => {
                let mut node = self.node(WidgetKind::Button);
                text(&mut node, "text", t)?;
                node
            }
            ("checkbox", [b, t]) => {
                let mut node = self.node(WidgetKind::Checkbox);
                binding(&mut node, b)?;
                text(&mut node, "text", t)?;
                node
            }
            ("radio_value" | "selectable_value", [b, v, t]) => {
                let kind = if method == "radio_value" {
                    WidgetKind::RadioValue
                } else {
                    WidgetKind::SelectableValue
                };
                let mut node = self.node(kind);
                binding(&mut node, b)?;
                node.props.insert("value", Value::Int(lit_int(v)?));
                text(&mut node, "text", t)?;
                node
            }
            ("text_edit_singleline" | "text_edit_multiline", [b]) => {
                let mut node = self.node(WidgetKind::TextEdit);
                binding(&mut node, b)?;
                node.props
                    .insert("multiline", Value::Bool(method == "text_edit_multiline"));
                node
            }
            // The short forms of a stack, and layouts it replaces.
            (
                "vertical" | "horizontal" | "horizontal_wrapped" | "vertical_centered" | "indent",
                [c],
            ) => {
                let mut node = container(self, WidgetKind::Stack, c)?;
                let row = method.starts_with("horizontal");
                let direction = if row { "row" } else { "column" };
                node.props
                    .insert("direction", Value::Text(direction.into()));
                match method.as_str() {
                    "horizontal_wrapped" => {
                        node.props.insert("wrap", Value::Bool(true));
                    }
                    "vertical_centered" => {
                        node.props.insert("align", Value::Text("Center".into()));
                    }
                    "indent" => {
                        // `spacing.indent`, 18 points by default.
                        node.props.insert("margin", Value::Size(18.0, 0.0));
                    }
                    _ => {}
                }
                node
            }
            ("collapsing", [t, c]) => {
                let mut node = container(self, WidgetKind::Collapsing, c)?;
                text(&mut node, "text", t)?;
                node
            }
            ("stack_item", [item, c]) => {
                let item = chain(item)?;
                if (item.ty.as_str(), item.ctor.as_str()) != ("StackItem", "new")
                    || !item.args.is_empty()
                {
                    return None;
                }
                let mut node = container(self, WidgetKind::StackItem, c)?;
                apply_builders(&mut node, &item.calls)?;
                node
            }
            ("grid_span", [columns, rows, c]) => {
                let mut node = container(self, WidgetKind::GridCell, c)?;
                cell_index(&mut node, "column_span", columns)?;
                cell_index(&mut node, "row_span", rows)?;
                node
            }
            ("grid_cell", [cell, c]) => {
                let cell = chain(cell)?;
                let [column, row] = cell.args.iter().collect::<Vec<_>>()[..] else {
                    return None;
                };
                if (cell.ty.as_str(), cell.ctor.as_str()) != ("GridCell", "at") {
                    return None;
                }
                let mut node = container(self, WidgetKind::GridCell, c)?;
                cell_index(&mut node, "column", column)?;
                cell_index(&mut node, "row", row)?;
                match cell.calls[..] {
                    [] => {}
                    [span] if span.method == "span" && span.args.len() == 2 => {
                        cell_index(&mut node, "column_span", &span.args[0])?;
                        cell_index(&mut node, "row_span", &span.args[1])?;
                    }
                    _ => return None,
                }
                node
            }
            ("add", [widget]) => self.widget(widget, ui)?,
            ("add_sized", [size, widget]) => {
                let mut node = self.widget(widget, ui)?;
                let (w, h) = lit_size(size)?;
                let value = Value::Size(w, h);
                let spec = node.kind.prop("fixed_size")?;
                if !spec.kind.accepts(&value) {
                    return None;
                }
                node.props.insert(spec.key, value);
                node
            }
            _ => return None,
        };
        Some(node)
    }

    /// The argument of `ui.add(…)`.
    fn widget(&mut self, e: &Expr, ui: &str) -> Option<Node> {
        let mut chain = chain(e)?;
        let args: Vec<&Expr> = chain.args.iter().collect();
        let mut node = match (chain.ty.as_str(), chain.ctor.as_str(), &args[..]) {
            ("Label", "new", [t]) => {
                // `.style(ui.style().heading.clone())`: a heading.
                let heading = format!("{ui}.style().heading.clone()");
                let style = chain.calls.iter().position(|c| {
                    c.method == "style" && c.args.len() == 1 && compact(&c.args[0]) == heading
                });
                let kind = match style {
                    Some(i) => {
                        chain.calls.remove(i);
                        WidgetKind::Heading
                    }
                    None => WidgetKind::Label,
                };
                let mut node = self.node(kind);
                node.props.insert("text", Value::Text(lit_str(t)?));
                node
            }
            ("Hyperlink", "new", [url]) => {
                let mut node = self.node(WidgetKind::Hyperlink);
                node.props.insert("text", Value::Text(String::new()));
                node.props.insert("url", Value::Text(lit_str(url)?));
                node
            }
            ("Hyperlink", "from_label_and_url", [t, url]) => {
                let mut node = self.node(WidgetKind::Hyperlink);
                node.props.insert("text", Value::Text(lit_str(t)?));
                node.props.insert("url", Value::Text(lit_str(url)?));
                node
            }
            ("Button", "new", [t]) => {
                let mut node = self.node(WidgetKind::Button);
                node.props.insert("text", Value::Text(lit_str(t)?));
                node
            }
            ("Checkbox", "new", [b, t]) => {
                let mut node = self.node(WidgetKind::Checkbox);
                node.props
                    .insert("binding", Value::Text(mut_self_field(b)?));
                node.props.insert("text", Value::Text(lit_str(t)?));
                node
            }
            ("TimePicker", "new", [hour, minute]) => {
                let binding = mut_self_field_index(hour, 0)?;
                if mut_self_field_index(minute, 1)? != binding {
                    return None;
                }
                let mut node = self.node(WidgetKind::TimePicker);
                node.props.insert("binding", Value::Text(binding));
                node
            }
            ("Slider", "new", [b, range]) => {
                let mut node = self.node(WidgetKind::Slider);
                node.props
                    .insert("binding", Value::Text(mut_self_field(b)?));
                let (min, max) = lit_range(range)?;
                node.props.insert("range", Value::Range(min, max));
                node
            }
            ("DragValue", "new", [b]) => {
                let mut node = self.node(WidgetKind::DragValue);
                node.props
                    .insert("binding", Value::Text(mut_self_field(b)?));
                node
            }
            ("ProgressBar", "new", [b]) => {
                let mut node = self.node(WidgetKind::ProgressBar);
                node.props.insert("binding", Value::Text(self_field(b)?));
                node
            }
            ("Spinner", "new", []) => self.node(WidgetKind::Spinner),
            ("Separator", "new", []) => self.node(WidgetKind::Separator),
            ("ColorPicker", "new", [b]) => {
                let mut node = self.node(WidgetKind::ColorPicker);
                node.props
                    .insert("binding", Value::Text(mut_self_field(b)?));
                node
            }
            ("DatePicker", "new", [b]) => {
                let mut node = self.node(WidgetKind::DatePicker);
                node.props
                    .insert("binding", Value::Text(mut_self_field(b)?));
                node
            }
            ("TextEdit", "singleline" | "multiline", [b]) => {
                let mut node = self.node(WidgetKind::TextEdit);
                node.props
                    .insert("binding", Value::Text(mut_self_field(b)?));
                node.props
                    .insert("multiline", Value::Bool(chain.ctor == "multiline"));
                node
            }
            ("SearchField", "new", [b]) => {
                let mut node = self.node(WidgetKind::SearchField);
                node.props
                    .insert("binding", Value::Text(mut_self_field(b)?));
                node
            }
            _ => return None,
        };
        apply_builders(&mut node, &chain.calls)?;
        Some(node)
    }
}

/// Applies `.method(arg)` calls to the node's properties; `None` if one
/// isn't known for its kind or its argument isn't a valid literal.
fn apply_builders(node: &mut Node, calls: &[&ExprMethodCall]) -> Option<()> {
    let table = builder_methods(node.kind);
    for call in calls {
        if call.turbofish.is_some() {
            return None;
        }
        let &(_, key, arg) = table.iter().find(|(m, _, _)| call.method == m)?;
        let args: Vec<&Expr> = call.args.iter().collect();
        let value = match (arg, &args[..]) {
            (Arg::Ignore, _) => continue,
            (Arg::Flag, []) => Value::Bool(true),
            (Arg::Set(text), []) => Value::Text(text.to_owned()),
            (Arg::Shadow, [e]) => Value::Shadow(lit_shadow(e)?),
            (Arg::Str, [e]) => Value::Text(lit_str(e)?),
            (Arg::Float, [e]) => Value::Float(lit_float(e)?),
            (Arg::Int, [e]) => Value::Int(lit_int(e)?),
            (Arg::Bool, [e]) => Value::Bool(lit_bool(e)?),
            (Arg::Range, [e]) => {
                let (min, max) = lit_range(e)?;
                Value::Range(min, max)
            }
            (Arg::Color, [e]) => Value::Color(lit_color(e)?),
            (Arg::Size, [e]) => {
                let (w, h) = lit_size(e)?;
                Value::Size(w, h)
            }
            (Arg::Stroke, [e]) => {
                let (width, color) = lit_stroke(e)?;
                Value::Stroke(width, color)
            }
            (Arg::Font, [e]) => Value::Font(lit_font(e)?),
            (Arg::Variant(ty), [e]) => Value::Text(variant(e, ty)?.to_string()),
            (Arg::Sizing, [e]) => Value::Sizing(lit_sizing(e)?),
            (Arg::Fill, []) => Value::Sizing(Sizing::Fill),
            (Arg::Tracks, [e]) => Value::Text(format_tracks(&lit_tracks(e)?)),
            (Arg::AxisLabel, [e]) => Value::Text(lit_axis_label(e)?),
            (Arg::LineStyle, [e]) => Value::Text(lit_line_style(e)?.to_owned()),
            _ => return None,
        };
        let spec = node.kind.prop(key)?;
        if !spec.kind.accepts(&value) {
            return None;
        }
        node.props.insert(spec.key, value);
    }
    Some(())
}

/// The builder calls of a `Container`, `Flex` or `FlexItem` written for
/// rustroke 0.19, as stack, stack item or grid properties.
fn legacy_builders(node: &mut Node, calls: &[&ExprMethodCall]) -> Option<()> {
    for call in calls {
        let args: Vec<&Expr> = call.args.iter().collect();
        let method = call.method.to_string();
        let fill = |e: &Expr| {
            Some(Value::Sizing(if lit_bool(e)? {
                Sizing::Fill
            } else {
                Sizing::Content
            }))
        };
        let (key, value) = match (node.kind, method.as_str(), &args[..]) {
            (WidgetKind::Stack, "inner_margin", [e]) => ("padding", size_value(e)?),
            (WidgetKind::Stack, "outer_margin", [e]) => ("margin", size_value(e)?),
            (WidgetKind::Stack, "fill_width", [e]) => ("width", fill(e)?),
            (WidgetKind::Stack, "fill_height", [e]) => ("height", fill(e)?),
            (WidgetKind::Stack | WidgetKind::StackItem, "justify" | "align", [e]) => {
                let ty = if method == "justify" {
                    "FlexJustify"
                } else {
                    "FlexAlign"
                };
                let key = if method == "justify" {
                    "justify"
                } else {
                    "align"
                };
                (key, Value::Text(variant(e, ty)?.to_string()))
            }
            (WidgetKind::Grid, "gap", [e]) => {
                let gap = lit_float(e)?;
                ("spacing", Value::Size(gap, gap))
            }
            _ => {
                // Same name and meaning as before.
                apply_builders(node, &[call])?;
                continue;
            }
        };
        node.props.insert(key, value);
    }
    Some(())
}

fn size_value(e: &Expr) -> Option<Value> {
    let (w, h) = lit_size(e)?;
    Some(Value::Size(w, h))
}

/// A grid cell's column, row or span from an integer literal.
fn cell_index(node: &mut Node, key: &'static str, e: &Expr) -> Option<()> {
    let value = Value::Int(lit_int(e)?);
    node.kind
        .prop(key)?
        .kind
        .accepts(&value)
        .then(|| node.props.insert(key, value))
        .map(|_| ())
}

/// The options of a combo box: `ui.selectable_value(&mut self.b, i, "item");`
/// with `i` counting from 0 and one binding for all.
fn combo_items(body: &Block, ui: &str, selected: Option<String>) -> Option<(String, Vec<String>)> {
    let mut binding = selected;
    let mut items = Vec::new();
    for (i, stmt) in body.stmts.iter().enumerate() {
        let Stmt::Expr(Expr::MethodCall(call), Some(_)) = stmt else {
            return None;
        };
        let args: Vec<&Expr> = call.args.iter().collect();
        if !is_ident(&call.receiver, ui) || call.method != "selectable_value" || args.len() != 3 {
            return None;
        }
        let b = mut_self_field(args[0])?;
        if *binding.get_or_insert_with(|| b.clone()) != b || lit_int(args[1])? != i as i64 {
            return None;
        }
        items.push(lit_str(args[2])?);
    }
    let binding = binding?;
    PropKind::Binding(crate::value::FieldType::Usize)
        .accepts(&Value::Text(binding.clone()))
        .then_some((binding, items))
}

/// The names in `#[derive(…)]` attributes.
fn derives(attrs: &[syn::Attribute]) -> Vec<String> {
    let mut out = Vec::new();
    for attr in attrs.iter().filter(|a| a.path().is_ident("derive")) {
        let _ = attr.parse_nested_meta(|meta| {
            let path = meta
                .path
                .segments
                .iter()
                .map(|s| s.ident.to_string())
                .collect::<Vec<_>>()
                .join("::");
            out.push(path);
            Ok(())
        });
    }
    out
}

/// The plain names of `use rustroke::Name;` / `use rustroke::{A, B};`;
/// `None` for any other item (kept as written).
fn rustroke_import(item: &Item) -> Option<Vec<String>> {
    let Item::Use(u) = item else {
        return None;
    };
    if u.leading_colon.is_some() || !u.attrs.is_empty() {
        return None;
    }
    let UseTree::Path(path) = &u.tree else {
        return None;
    };
    if path.ident != "rustroke" {
        return None;
    }
    match &*path.tree {
        UseTree::Name(name) => Some(vec![name.ident.to_string()]),
        UseTree::Group(group) => group
            .items
            .iter()
            .map(|tree| match tree {
                UseTree::Name(name) => Some(name.ident.to_string()),
                _ => None,
            })
            .collect(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::view_code;
    use crate::model::NodeId;

    /// The view with every id set to 0, to compare structure.
    fn without_ids(view: &View) -> (Vec<Node>, View) {
        let mut nodes = view.nodes.clone();
        for node in &mut nodes {
            node.walk_mut(&mut |n| n.id = NodeId(0));
        }
        let mut rest = View::new(view.name.clone());
        rest.window = view.window.clone();
        rest.style = view.style.clone();
        rest.derives = view.derives.clone();
        rest.extra_fields = view.extra_fields.clone();
        rest.extra_methods = view.extra_methods.clone();
        rest.extra_items = view.extra_items.clone();
        rest.extra_imports = view.extra_imports.clone();
        (nodes, rest)
    }

    #[test]
    fn sample_views_round_trip() {
        for view in crate::sample_project().views {
            let code = view_code(&view);
            let parsed = parse_view(&view.name, &code).unwrap_or_else(|e| panic!("{e}\n{code}"));
            let (expected, rest) = without_ids(&view);
            let (actual, parsed_rest) = without_ids(&parsed);
            for (e, a) in expected.iter().zip(&actual) {
                assert_eq!(e, a, "in view {}", view.name);
            }
            assert_eq!(expected.len(), actual.len(), "in view {}", view.name);
            assert_eq!(rest, parsed_rest);
            assert_eq!(view_code(&parsed), code, "regenerated code is stable");
        }
    }

    #[test]
    fn hand_written_code() {
        let code = r#"
use rustroke::{Slider, Ui};
use std::fmt::Write as _;

// A note about the struct.
#[derive(Debug, Default, Clone)]
pub struct Form {
    pub name: String,
    pub volume: f32,
    #[allow(dead_code)]
    cache: Vec<u8>,
}

impl Form {
    pub fn show(&mut self, ui: &mut Ui<'_>) {
        // Title.
        ui.heading("Form");
        ui.text_edit_singleline(&mut self.name);
        let greeting = format!("Hello {}", self.name);
        ui.label(greeting);
        ui.add(Slider::new(&mut self.volume, 0.0..=10.0).text("Volume").unknown(3));
        ui.horizontal(|row| {
            row.label("In a row");
            if row.button("Ok").clicked() { self.volume = 0.0; }
        });
        // Trailing comment.
    }

    fn helper(&self) -> usize {
        self.name.len()
    }
}
"#;
        let view = parse_view("form", code).unwrap();
        let kinds: Vec<_> = view.nodes.iter().map(|n| n.kind).collect();
        use WidgetKind as K;
        assert_eq!(
            kinds,
            [
                K::RawCode,
                K::Heading,
                K::TextEdit,
                K::RawCode,
                K::Stack,
                K::RawCode
            ]
        );
        assert_eq!(view.nodes[0].text("code"), "// Title.");
        // Consecutive free statements form one node.
        let free = view.nodes[3].text("code");
        assert!(
            free.starts_with(
                "let greeting = format!(\"Hello {}\", self.name);\nui.label(greeting);\n"
            ),
            "{free}"
        );
        assert!(free.ends_with(".unknown(3));"), "{free}");
        let row = &view.nodes[4].children;
        assert_eq!(row[0].text("text"), "In a row");
        assert_eq!(row[1].text("on_click"), "self.volume = 0.0;");
        assert_eq!(view.nodes[5].text("code"), "// Trailing comment.");
        assert_eq!(view.derives, ["Debug", "Default", "Clone"]);
        assert_eq!(
            view.extra_fields.len(),
            2,
            "volume is only used by free code"
        );
        assert_eq!(
            view.extra_fields[1].code,
            "#[allow(dead_code)]\ncache: Vec<u8>"
        );
        assert_eq!(
            view.extra_methods,
            ["fn helper(&self) -> usize {\n    self.name.len()\n}"]
        );
        assert_eq!(view.extra_items, ["use std::fmt::Write as _;"]);
        assert!(view.extra_imports.is_empty());
        // The generated code reads back the same way.
        let again = parse_view("form", &view_code(&view)).unwrap();
        assert_eq!(without_ids(&again).0.len(), view.nodes.len());
    }

    #[test]
    fn layouts_of_rustroke_0_19_become_stacks_and_grids() {
        let code = r#"
use rustroke::{Container, Flex, FlexAlign, FlexGrid, FlexItem, FlexJustify, GridCell, Track, Ui, vec2};

pub struct Old {}

impl Old {
    pub fn show(&mut self, ui: &mut Ui<'_>) {
        Container::new()
            .corner_radius(8.0)
            .inner_margin(vec2(12.0, 10.0))
            .outer_margin(vec2(0.0, 6.0))
            .fill_width(true)
            .show(ui, |ui| {
                ui.label("Card");
            });
        Flex::row("cards")
            .gap(12.0)
            .justify(FlexJustify::SpaceBetween)
            .align(FlexAlign::Stretch)
            .fill_height(true)
            .show(ui, |flex| {
                flex.add(FlexItem::new().grow(1.0).align(FlexAlign::Center), |ui| {
                    ui.label("Item");
                });
            });
        FlexGrid::new("form", vec![Track::Points(120.0), Track::Fraction(1.0)])
            .gap(6.0)
            .show(ui, |grid| {
                grid.add(GridCell::at(0, 1).span(2, 1), |ui| {
                    ui.button("Save");
                });
            });
    }
}
"#;
        use WidgetKind as K;
        let view = parse_view("old", code).unwrap();
        let kinds: Vec<_> = view.nodes.iter().map(|n| n.kind).collect();
        assert_eq!(kinds, [K::Stack, K::Stack, K::Grid]);
        let card = &view.nodes[0];
        assert_eq!(card.text("direction"), "column");
        assert_eq!(card.prop("padding"), Some(&Value::Size(12.0, 10.0)));
        assert_eq!(card.prop("margin"), Some(&Value::Size(0.0, 6.0)));
        assert_eq!(card.prop("width"), Some(&Value::Sizing(Sizing::Fill)));
        assert_eq!(card.children[0].kind, K::Label);
        let row = &view.nodes[1];
        assert_eq!(row.text("direction"), "row");
        assert_eq!(row.text("justify"), "SpaceBetween");
        assert_eq!(row.text("align"), "Stretch");
        assert_eq!(row.prop("height"), Some(&Value::Sizing(Sizing::Fill)));
        let item = &row.children[0];
        assert_eq!((item.kind, item.text("align")), (K::StackItem, "Center"));
        assert_eq!(item.prop("grow"), Some(&Value::Float(1.0)));
        let grid = &view.nodes[2];
        assert_eq!(grid.text("columns"), "120, 1fr");
        assert_eq!(grid.prop("spacing"), Some(&Value::Size(6.0, 6.0)));
        let cell = &grid.children[0];
        assert_eq!(cell.kind, K::GridCell);
        assert_eq!(cell.prop("row"), Some(&Value::Int(1)));
        assert_eq!(cell.prop("column_span"), Some(&Value::Int(2)));
        // Written back with the new layouts, without the old imports.
        let new = view_code(&view);
        assert!(new.contains("Stack::row()"), "{new}");
        assert!(new.contains("ui.grid_cell("), "{new}");
        assert!(new.contains("GridCell::at(0, 1).span(2, 1)"), "{new}");
        assert!(!new.contains("Flex"), "{new}");
    }

    /// Small deterministic random generator (xorshift).
    struct Rng(u64);

    impl Rng {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
        fn text(&mut self) -> String {
            const PIECES: &[&str] = &[
                "a", "Save", " ", "\"", "\\", "é", "{}", "\n", "\t", "日本", "#",
            ];
            (0..self.below(4))
                .map(|_| PIECES[self.below(PIECES.len())])
                .collect()
        }
        /// Exactly representable as `f32` and printed without loss.
        fn float(&mut self) -> f64 {
            (self.below(4000) as f64 - 2000.0) / 4.0
        }
        fn color(&mut self) -> [u8; 4] {
            let alpha = if self.below(2) == 0 {
                255
            } else {
                self.below(256) as u8
            };
            [
                self.below(256) as u8,
                self.below(256) as u8,
                self.below(256) as u8,
                alpha,
            ]
        }
        fn value(&mut self, kind: PropKind) -> Option<Value> {
            Some(match kind {
                PropKind::Color => Value::Color(self.color()),
                PropKind::Size => Value::Size(self.float().abs(), self.float().abs()),
                PropKind::Limit => {
                    let mut side = || match self.below(3) {
                        0 => f64::INFINITY,
                        _ => self.float().abs(),
                    };
                    Value::Size(side(), side())
                }
                PropKind::Shadow => Value::Shadow(Shadow {
                    offset: (self.float(), self.float()),
                    blur: self.float().abs(),
                    spread: self.float(),
                    color: self.color(),
                }),
                PropKind::Stroke => Value::Stroke(self.float().abs(), self.color()),
                PropKind::Font => Value::Font(Font {
                    family: FontFamily::ALL[self.below(3)],
                    size: self.float().abs() + 1.0,
                    weight: [300, 400, 600, 700][self.below(4)],
                    bold: self.below(2) == 0,
                }),
                PropKind::Sizing => Value::Sizing(match self.below(3) {
                    0 => Sizing::Content,
                    1 => Sizing::Fill,
                    _ => Sizing::Points(self.float().abs()),
                }),
                PropKind::Tracks => {
                    const TRACKS: &[&str] = &["120, 1fr", "auto", "50, 2.5fr, auto", "1fr, 1fr"];
                    Value::Text(TRACKS[self.below(TRACKS.len())].to_owned())
                }
                _ => return None,
            })
        }
    }

    const FREE_CODE: &[&str] = &[
        "// A comment.",
        "let x = 1;\nlet _ = x + 1;",
        "if self.flag {\n    // Nested.\n    ui.label(\"on\");\n}",
        "for i in 0..3 {\n    ui.label(i.to_string());\n}",
    ];

    fn random_node(
        rng: &mut Rng,
        view: &mut View,
        parent: Option<WidgetKind>,
        depth: usize,
    ) -> Node {
        let kind = loop {
            let kind = WidgetKind::ALL[rng.below(WidgetKind::ALL.len())];
            if WidgetKind::accepts(parent, kind)
                && (depth < 3
                    || !kind.is_container()
                    || matches!(kind, WidgetKind::StackItem | WidgetKind::GridCell))
            {
                break kind;
            }
        };
        let mut node = view.create(kind);
        for spec in kind.spec().props {
            if spec.optional() && rng.below(2) == 0 {
                continue;
            }
            let value = match spec.kind {
                PropKind::Text => Value::Text(rng.text()),
                PropKind::IdSalt => Value::Text(format!("id{}", rng.below(5))),
                PropKind::Bool => Value::Bool(rng.below(2) == 0),
                PropKind::Float { min, max } => Value::Float(if min >= 0.0 {
                    rng.float().abs().min(max)
                } else {
                    rng.float()
                }),
                PropKind::Int { min, max } => Value::Int((min + rng.below(10) as i64).min(max)),
                PropKind::Range => Value::Range(rng.float(), rng.float()),
                PropKind::Items => Value::Items((0..rng.below(4)).map(|_| rng.text()).collect()),
                PropKind::Binding(_) => Value::Text(["a", "b", "value"][rng.below(3)].to_owned()),
                PropKind::Choice(choices) => {
                    Value::Text(choices[rng.below(choices.len())].to_owned())
                }
                PropKind::Code => Value::Text(if kind == WidgetKind::Button && rng.below(2) == 0 {
                    String::new()
                } else {
                    FREE_CODE[rng.below(FREE_CODE.len())].to_owned()
                }),
                other => rng.value(other).expect("every kind of value"),
            };
            node.props.insert(spec.key, value);
        }
        // A cell is placed by its column and row together, or not at all.
        if kind == WidgetKind::GridCell
            && node.prop("column").is_some() != node.prop("row").is_some()
        {
            node.props.remove("column");
            node.props.remove("row");
        }
        if kind.is_container() {
            node.children = random_children(rng, view, Some(kind), depth + 1);
        }
        node
    }

    fn random_children(
        rng: &mut Rng,
        view: &mut View,
        parent: Option<WidgetKind>,
        depth: usize,
    ) -> Vec<Node> {
        let mut children: Vec<Node> = Vec::new();
        for _ in 0..rng.below(5) {
            let node = random_node(rng, view, parent, depth);
            // Adjacent free code merges into one node when read back.
            let last_raw = children
                .last()
                .is_some_and(|n| n.kind == WidgetKind::RawCode);
            if !(last_raw && node.kind == WidgetKind::RawCode) {
                children.push(node);
            }
        }
        children
    }

    #[test]
    fn random_views_round_trip() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let mut total = 0;
        let mut kinds = std::collections::BTreeSet::new();
        for round in 0..600 {
            let mut view = View::new("random");
            for spec in CONTENT_STYLE {
                if rng.below(4) == 0 {
                    view.style.insert(spec.key, rng.value(spec.kind).unwrap());
                }
            }
            view.nodes = random_children(&mut rng, &mut view, None, 0);
            let code = view_code(&view);
            let parsed = parse_view("random", &code)
                .unwrap_or_else(|e| panic!("round {round}: {e}\n{code}"));
            assert_eq!(
                without_ids(&parsed).0,
                without_ids(&view).0,
                "round {round}:\n{code}"
            );
            assert_eq!(parsed.style, view.style, "round {round}:\n{code}");
            view.walk(|n| {
                total += 1;
                kinds.insert(n.kind);
            });
        }
        assert!(total > 2000, "only {total} nodes generated");
        assert_eq!(kinds.len(), WidgetKind::ALL.len(), "every kind is tried");
    }

    #[test]
    fn errors_have_positions() {
        let err = parse_view("x", "pub struct X {}\nimpl X {\n    pub fn show(&mut self) {\n        ui.label(\"a\")\n        ui.label(\"b\");\n    }\n}\n")
            .unwrap_err();
        assert_eq!(err.line, 5, "{err}");
        let err = parse_view("x", "pub struct X {}\n").unwrap_err();
        assert!(err.message.contains("impl X"), "{err}");
    }
}
