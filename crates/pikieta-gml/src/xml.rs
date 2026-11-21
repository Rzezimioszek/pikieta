//! Bezstratne drzewo XML.
//!
//! Parser zachowuje dokładny tekst każdego znacznika, tekstu, komentarza i deklaracji,
//! więc dokument zapisany bez zmian jest identyczny bajt w bajt z wczytanym.
//!
//! Pamięć przy dużych plikach (setki MB): nazwy elementów i atrybutów oraz teksty
//! złożone z samych białych znaków są współdzielone (`Arc<str>`), a znaczniki w postaci
//! kanonicznej (`<a b="c">`, `</a>`) nie są przechowywane, tylko odtwarzane przy zapisie.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

/// Węzeł drzewa.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Element(Box<Element>),
    /// Tekst w postaci z pliku (z encjami, np. `&amp;`).
    Text(Arc<str>),
    /// Sekcja CDATA: sama zawartość, bez `<![CDATA[` i `]]>`.
    CData(Arc<str>),
    /// Komentarz, instrukcja przetwarzania, deklaracja XML, DOCTYPE: dokładny tekst.
    Other(Arc<str>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attr {
    pub name: Arc<str>,
    /// Wartość w postaci z pliku (z encjami).
    pub raw: String,
}

/// Postać znacznika zamykającego.
#[derive(Debug, Clone, PartialEq)]
enum Close {
    /// Element pusty `<a/>`.
    Empty,
    /// `</a>`.
    Canonical,
    /// Inny zapis, np. `</a >`.
    Raw(Box<str>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub name: Arc<str>,
    pub attrs: Vec<Attr>,
    pub children: Vec<Node>,
    /// Oryginalny znacznik otwierający, gdy różni się od kanonicznego; `None` po zmianie atrybutów.
    open_raw: Option<Box<str>>,
    close: Close,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "wiersz {}: {}", self.line, self.message)
    }
}

impl std::error::Error for XmlError {}

impl Element {
    pub fn new(name: &str) -> Self {
        Self {
            name: Arc::from(name),
            attrs: Vec::new(),
            children: Vec::new(),
            open_raw: None,
            close: Close::Canonical,
        }
    }

    /// Nazwa bez przedrostka przestrzeni nazw.
    pub fn local_name(&self) -> &str {
        local(&self.name)
    }

    pub fn prefix(&self) -> &str {
        self.name.split_once(':').map_or("", |(p, _)| p)
    }

    /// Wartość atrybutu (po rozwinięciu encji) o pełnej nazwie, np. `gml:id`.
    pub fn attr(&self, name: &str) -> Option<String> {
        self.attrs
            .iter()
            .find(|a| &*a.name == name)
            .map(|a| unescape(&a.raw))
    }

    /// Wartość atrybutu o podanej nazwie lokalnej, z dowolnym przedrostkiem.
    pub fn attr_local(&self, local_name: &str) -> Option<String> {
        self.attrs
            .iter()
            .find(|a| local(&a.name) == local_name)
            .map(|a| unescape(&a.raw))
    }

    pub fn set_attr(&mut self, name: &str, value: &str) {
        let raw = escape(value);
        match self.attrs.iter_mut().find(|a| &*a.name == name) {
            Some(a) => a.raw = raw,
            None => self.attrs.push(Attr {
                name: Arc::from(name),
                raw,
            }),
        }
        self.open_raw = None;
    }

    pub fn elements(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|n| match n {
            Node::Element(e) => Some(e.as_ref()),
            _ => None,
        })
    }

    pub fn has_elements(&self) -> bool {
        self.elements().next().is_some()
    }

    /// Pierwsze dziecko o podanej nazwie lokalnej.
    pub fn child(&self, local_name: &str) -> Option<&Element> {
        self.elements().find(|e| e.local_name() == local_name)
    }

    /// Tekst elementu: połączone węzły tekstowe i CDATA, z rozwiniętymi encjami.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for n in &self.children {
            match n {
                Node::Text(t) => out.push_str(&unescape(t)),
                Node::CData(t) => out.push_str(t),
                _ => {}
            }
        }
        out
    }

    /// Zastępuje zawartość elementu tekstem. Pusty element `<a/>` dostaje znacznik zamykający.
    pub fn set_text(&mut self, value: &str) {
        self.children = vec![Node::Text(Arc::from(escape(value)))];
        if self.close == Close::Empty {
            self.close = Close::Canonical;
            self.open_raw = None;
        }
    }

    pub fn element_at(&self, path: &[usize]) -> Option<&Element> {
        let Some((&first, rest)) = path.split_first() else {
            return Some(self);
        };
        match self.children.get(first)? {
            Node::Element(e) => e.element_at(rest),
            _ => None,
        }
    }

    pub fn element_at_mut(&mut self, path: &[usize]) -> Option<&mut Element> {
        let Some((&first, rest)) = path.split_first() else {
            return Some(self);
        };
        match self.children.get_mut(first)? {
            Node::Element(e) => e.element_at_mut(rest),
            _ => None,
        }
    }

    fn write(&self, out: &mut String) {
        match &self.open_raw {
            Some(raw) => out.push_str(raw),
            None => {
                out.push('<');
                out.push_str(&self.name);
                for a in &self.attrs {
                    out.push(' ');
                    out.push_str(&a.name);
                    out.push_str("=\"");
                    // Wartość z apostrofów może zawierać cudzysłów; w kanonicznej postaci musi być encją.
                    if a.raw.contains('"') {
                        out.push_str(&a.raw.replace('"', "&quot;"));
                    } else {
                        out.push_str(&a.raw);
                    }
                    out.push('"');
                }
                out.push_str(if self.close == Close::Empty {
                    "/>"
                } else {
                    ">"
                });
            }
        }
        for n in &self.children {
            n.write(out);
        }
        match &self.close {
            Close::Empty => {}
            Close::Canonical => {
                out.push_str("</");
                out.push_str(&self.name);
                out.push('>');
            }
            Close::Raw(raw) => out.push_str(raw),
        }
    }
}

impl Node {
    fn write(&self, out: &mut String) {
        match self {
            Node::Element(e) => e.write(out),
            Node::Text(t) | Node::Other(t) => out.push_str(t),
            Node::CData(t) => {
                out.push_str("<![CDATA[");
                out.push_str(t);
                out.push_str("]]>");
            }
        }
    }
}

impl Document {
    /// Pierwszy element najwyższego poziomu.
    pub fn root(&self) -> Option<&Element> {
        self.nodes.iter().find_map(|n| match n {
            Node::Element(e) => Some(e.as_ref()),
            _ => None,
        })
    }

    pub fn root_mut(&mut self) -> Option<&mut Element> {
        self.nodes.iter_mut().find_map(|n| match n {
            Node::Element(e) => Some(e.as_mut()),
            _ => None,
        })
    }

    pub fn to_xml(&self) -> String {
        let mut out = String::new();
        for n in &self.nodes {
            n.write(&mut out);
        }
        out
    }
}

pub fn local(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, l)| l)
}

/// Rozwija encje XML. Nieznane encje zostają bez zmian.
pub fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let Some(end) = rest.find(';') else { break };
        let entity = &rest[1..end];
        let ch = match entity {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                u32::from_str_radix(&entity[2..], 16)
                    .ok()
                    .and_then(char::from_u32)
            }
            _ if entity.starts_with('#') => entity[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// Czy znacznik ma postać, którą `Element::write` odtworzy bajt w bajt.
fn is_canonical(tag: &str, name: &str, attrs: &[Attr], empty: bool) -> bool {
    let Some(mut rest) = tag.strip_prefix('<').and_then(|r| r.strip_prefix(name)) else {
        return false;
    };
    for a in attrs {
        if a.raw.contains('"') {
            return false;
        }
        let Some(r) = rest
            .strip_prefix(' ')
            .and_then(|r| r.strip_prefix(&*a.name))
            .and_then(|r| r.strip_prefix("=\""))
            .and_then(|r| r.strip_prefix(a.raw.as_str()))
            .and_then(|r| r.strip_prefix('"'))
        else {
            return false;
        };
        rest = r;
    }
    rest == if empty { "/>" } else { ">" }
}

struct Parser<'a> {
    src: &'a str,
    pos: usize,
    /// Współdzielone nazwy i teksty z białych znaków.
    shared: HashMap<&'a str, Arc<str>>,
}

impl<'a> Parser<'a> {
    fn line(&self, at: usize) -> usize {
        self.src[..at].bytes().filter(|&b| b == b'\n').count() + 1
    }

    fn err(&self, at: usize, message: impl Into<String>) -> XmlError {
        XmlError {
            line: self.line(at),
            message: message.into(),
        }
    }

    fn share(&mut self, s: &'a str) -> Arc<str> {
        self.shared.entry(s).or_insert_with(|| Arc::from(s)).clone()
    }

    fn text(&mut self, s: &'a str) -> Arc<str> {
        if s.len() <= 64 && s.bytes().all(|b| b.is_ascii_whitespace()) {
            self.share(s)
        } else {
            Arc::from(s)
        }
    }

    fn rest(&self) -> &'a str {
        &self.src[self.pos..]
    }

    /// Przesuwa za najbliższe wystąpienie `end` i zwraca tekst od `start` włącznie z `end`.
    fn take_until(&mut self, start: usize, end: &str, what: &str) -> Result<&'a str, XmlError> {
        match self.src[self.pos..].find(end) {
            Some(i) => {
                self.pos += i + end.len();
                Ok(&self.src[start..self.pos])
            }
            None => Err(self.err(start, format!("niezamknięty {what}"))),
        }
    }

    fn parse_doctype(&mut self, start: usize) -> Result<&'a str, XmlError> {
        let mut depth = 0usize;
        for (i, b) in self.src[self.pos..].bytes().enumerate() {
            match b {
                b'[' => depth += 1,
                b']' => depth = depth.saturating_sub(1),
                b'>' if depth == 0 => {
                    self.pos += i + 1;
                    return Ok(&self.src[start..self.pos]);
                }
                _ => {}
            }
        }
        Err(self.err(start, "niezamknięty DOCTYPE"))
    }

    /// Znacznik otwierający od `<` do `>` lub `/>`, z uwzględnieniem cudzysłowów.
    fn parse_start_tag(&mut self, start: usize) -> Result<(Element, bool), XmlError> {
        let src = self.src;
        let bytes = src.as_bytes();
        let mut i = start + 1;
        let name_start = i;
        while i < bytes.len() && !matches!(bytes[i], b' ' | b'\t' | b'\r' | b'\n' | b'>' | b'/') {
            i += 1;
        }
        let name = &src[name_start..i];
        if name.is_empty() {
            return Err(self.err(start, "pusta nazwa elementu"));
        }
        let mut attrs = Vec::new();
        loop {
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let empty = match bytes.get(i) {
                None => return Err(self.err(start, format!("niezamknięty znacznik <{name}>"))),
                Some(b'>') => Some(false),
                Some(b'/') if bytes.get(i + 1) == Some(&b'>') => Some(true),
                Some(_) => None,
            };
            if let Some(empty) = empty {
                i += if empty { 2 } else { 1 };
                self.pos = i;
                let tag = &src[start..i];
                let canonical = is_canonical(tag, name, &attrs, empty);
                let el = Element {
                    name: self.share(name),
                    attrs,
                    children: Vec::new(),
                    open_raw: (!canonical).then(|| Box::from(tag)),
                    close: if empty {
                        Close::Empty
                    } else {
                        Close::Canonical
                    },
                };
                return Ok((el, empty));
            }
            let an_start = i;
            while i < bytes.len()
                && !matches!(bytes[i], b'=' | b' ' | b'\t' | b'\r' | b'\n' | b'>' | b'/')
            {
                i += 1;
            }
            let an = &src[an_start..i];
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if bytes.get(i) != Some(&b'=') {
                return Err(self.err(start, format!("atrybut {an} bez wartości w <{name}>")));
            }
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            let quote = match bytes.get(i) {
                Some(q @ (b'"' | b'\'')) => *q,
                _ => return Err(self.err(start, format!("wartość atrybutu {an} bez cudzysłowu"))),
            };
            let v_start = i + 1;
            let Some(len) = src[v_start..].find(quote as char) else {
                return Err(self.err(start, format!("niezamknięta wartość atrybutu {an}")));
            };
            attrs.push(Attr {
                name: self.share(an),
                raw: src[v_start..v_start + len].to_string(),
            });
            i = v_start + len + 1;
        }
    }
}

/// Wczytuje dokument XML.
pub fn parse(src: &str) -> Result<Document, XmlError> {
    let mut p = Parser {
        src,
        pos: 0,
        shared: HashMap::new(),
    };
    let mut stack: Vec<Element> = Vec::new();
    let mut top: Vec<Node> = Vec::new();

    fn push(stack: &mut [Element], top: &mut Vec<Node>, node: Node) {
        match stack.last_mut() {
            Some(parent) => parent.children.push(node),
            None => top.push(node),
        }
    }

    while p.pos < src.len() {
        let start = p.pos;
        let rest = p.rest();
        if let Some(stripped) = rest.strip_prefix('<') {
            if stripped.starts_with("!--") {
                let raw = p.take_until(start, "-->", "komentarz")?;
                push(&mut stack, &mut top, Node::Other(Arc::from(raw)));
            } else if stripped.starts_with("![CDATA[") {
                p.take_until(start, "]]>", "blok CDATA")?;
                let inner = &src[start + 9..p.pos - 3];
                push(&mut stack, &mut top, Node::CData(Arc::from(inner)));
            } else if stripped.starts_with('?') {
                let raw = p.take_until(start, "?>", "instrukcja przetwarzania")?;
                push(&mut stack, &mut top, Node::Other(Arc::from(raw)));
            } else if stripped.starts_with('!') {
                p.pos += 2;
                let raw = p.parse_doctype(start)?;
                push(&mut stack, &mut top, Node::Other(Arc::from(raw)));
            } else if stripped.starts_with('/') {
                let raw = p.take_until(start, ">", "znacznik zamykający")?;
                let name = raw[2..raw.len() - 1].trim();
                let Some(mut open) = stack.pop() else {
                    return Err(p.err(start, format!("znacznik </{name}> bez otwarcia")));
                };
                if &*open.name != name {
                    return Err(p.err(
                        start,
                        format!("oczekiwano </{}>, jest </{name}>", open.name),
                    ));
                }
                if raw.len() != open.name.len() + 3 {
                    open.close = Close::Raw(Box::from(raw));
                }
                push(&mut stack, &mut top, Node::Element(Box::new(open)));
            } else {
                let (el, empty) = p.parse_start_tag(start)?;
                if empty {
                    push(&mut stack, &mut top, Node::Element(Box::new(el)));
                } else {
                    stack.push(el);
                }
            }
        } else {
            let len = rest.find('<').unwrap_or(rest.len());
            p.pos += len;
            let t = p.text(&rest[..len]);
            push(&mut stack, &mut top, Node::Text(t));
        }
    }
    if let Some(open) = stack.last() {
        return Err(p.err(src.len(), format!("niezamknięty element <{}>", open.name)));
    }
    let doc = Document { nodes: top };
    if doc.root().is_none() {
        return Err(XmlError {
            line: 1,
            message: "brak elementu głównego".into(),
        });
    }
    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\u{feff}<?xml version=\"1.0\" encoding=\"UTF-8\"?>\r\n\
<!-- komentarz -->\n<a:root xmlns:a=\"urn:x\"  b = 'jeden &amp; dwa' >\n\
  <a:item id=\"1\">tekst &lt;ok&gt; &#261;</a:item>\n  <a:pusty/>\n  <a:pusty2 />\n\
  <a:dane><![CDATA[<surowe>]]></a:dane >\n  <a:q x='z \"cytatem\"'/>\n</a:root>\n";

    #[test]
    fn round_trip_is_byte_identical() {
        let doc = parse(SAMPLE).unwrap();
        assert_eq!(doc.to_xml(), SAMPLE);
    }

    #[test]
    fn reads_values() {
        let doc = parse(SAMPLE).unwrap();
        let root = doc.root().unwrap();
        assert_eq!(root.local_name(), "root");
        assert_eq!(root.prefix(), "a");
        assert_eq!(root.attr("b").unwrap(), "jeden & dwa");
        assert_eq!(root.child("item").unwrap().text(), "tekst <ok> ą");
        assert_eq!(root.child("dane").unwrap().text(), "<surowe>");
        assert_eq!(root.child("item").unwrap().attr_local("id").unwrap(), "1");
        assert_eq!(root.child("q").unwrap().attr("x").unwrap(), "z \"cytatem\"");
        assert_eq!(root.elements().count(), 5);
    }

    #[test]
    fn edits_touch_only_changed_elements() {
        let mut doc = parse(SAMPLE).unwrap();
        let root = doc.root_mut().unwrap();
        root.element_at_mut(&[1]).unwrap().set_text("a < b");
        root.element_at_mut(&[3]).unwrap().set_text("nowy");
        root.element_at_mut(&[9]).unwrap().set_attr("y", "1");
        let xml = doc.to_xml();
        assert!(xml.contains("<a:item id=\"1\">a &lt; b</a:item>"));
        assert!(xml.contains("<a:pusty>nowy</a:pusty>"));
        assert!(xml.contains("<a:pusty2 />"));
        assert!(xml.contains("b = 'jeden &amp; dwa' >"));
        assert!(xml.contains("</a:dane >"));
        assert!(xml.contains("<a:q x=\"z &quot;cytatem&quot;\" y=\"1\"/>"));
        let again = parse(&xml).unwrap();
        let root = again.root().unwrap();
        assert_eq!(root.child("item").unwrap().text(), "a < b");
        assert_eq!(root.child("q").unwrap().attr("x").unwrap(), "z \"cytatem\"");
    }

    #[test]
    fn errors_have_line_numbers() {
        let e = parse("<a>\n<b>\n</a>").unwrap_err();
        assert_eq!(e.line, 3);
        assert!(e.message.contains("oczekiwano </b>"));
        assert!(parse("<a x=1/>").is_err());
        assert!(parse("<a>").unwrap_err().message.contains("niezamknięty"));
        assert!(parse("tylko tekst").is_err());
    }
}
