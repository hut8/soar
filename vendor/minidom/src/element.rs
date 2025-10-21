// Copyright (c) 2020 lumi <lumi@pew.im>
// Copyright (c) 2020 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
// Copyright (c) 2020 Bastien Orivel <eijebong+minidom@bananium.fr>
// Copyright (c) 2020 Maxime “pep” Buquet <pep@bouah.net>
// Copyright (c) 2020 Yue Liu <amznyue@amazon.com>
// Copyright (c) 2020 Matt Bilker <me@mbilker.us>
// Copyright (c) 2020 Xidorn Quan <me@upsuper.org>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Provides an `Element` type, which represents DOM nodes, and a builder to create them with.

use crate::convert::IntoAttributeValue;
use crate::error::{Error, Result};
use crate::namespace_set::{NSChoice, NamespaceSet};
use crate::node::Node;

use std::collections::{btree_map, BTreeMap};
use std::io::Write;

use std::borrow::Cow;
use std::rc::Rc;
use std::str;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, Event};
use quick_xml::Reader as EventReader;
use quick_xml::Writer as EventWriter;

use std::io::BufRead;

use std::str::FromStr;

use std::slice;

/// helper function to escape a `&[u8]` and replace all
/// xml special characters (<, >, &, ', ") with their corresponding
/// xml escaped value.
pub fn escape(raw: &[u8]) -> Cow<[u8]> {
    let mut escapes: Vec<(usize, &'static [u8])> = Vec::new();
    let mut bytes = raw.iter();
    fn to_escape(b: u8) -> bool {
        match b {
            b'<' | b'>' | b'\'' | b'&' | b'"' => true,
            _ => false,
        }
    }

    let mut loc = 0;
    while let Some(i) = bytes.position(|&b| to_escape(b)) {
        loc += i;
        match raw[loc] {
            b'<' => escapes.push((loc, b"&lt;")),
            b'>' => escapes.push((loc, b"&gt;")),
            b'\'' => escapes.push((loc, b"&apos;")),
            b'&' => escapes.push((loc, b"&amp;")),
            b'"' => escapes.push((loc, b"&quot;")),
            _ => unreachable!("Only '<', '>','\', '&' and '\"' are escaped"),
        }
        loc += 1;
    }

    if escapes.is_empty() {
        Cow::Borrowed(raw)
    } else {
        let len = raw.len();
        let mut v = Vec::with_capacity(len);
        let mut start = 0;
        for (i, r) in escapes {
            v.extend_from_slice(&raw[start..i]);
            v.extend_from_slice(r);
            start = i + 1;
        }

        if start < len {
            v.extend_from_slice(&raw[start..]);
        }
        Cow::Owned(v)
    }
}

#[derive(Clone, Eq, Debug)]
/// A struct representing a DOM Element.
pub struct Element {
    prefix: Option<String>,
    name: String,
    namespaces: Rc<NamespaceSet>,
    attributes: BTreeMap<String, String>,
    children: Vec<Node>,
}

impl<'a> From<&'a Element> for String {
    fn from(elem: &'a Element) -> String {
        let mut writer = Vec::new();
        elem.write_to(&mut writer).unwrap();
        String::from_utf8(writer).unwrap()
    }
}

impl FromStr for Element {
    type Err = Error;

    fn from_str(s: &str) -> Result<Element> {
        let mut reader = EventReader::from_str(s);
        Element::from_reader(&mut reader)
    }
}

impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        if self.name() == other.name() && self.ns() == other.ns() && self.attrs().eq(other.attrs())
        {
            self.nodes()
                .zip(other.nodes())
                .all(|(node1, node2)| node1 == node2)
        } else {
            false
        }
    }
}

impl Element {
    fn new<NS: Into<NamespaceSet>>(
        name: String,
        prefix: Option<String>,
        namespaces: NS,
        attributes: BTreeMap<String, String>,
        children: Vec<Node>,
    ) -> Element {
        Element {
            prefix,
            name,
            namespaces: Rc::new(namespaces.into()),
            attributes,
            children,
        }
    }

    /// Return a builder for an `Element` with the given `name`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let elem = Element::builder("name")
    ///                    .ns("namespace")
    ///                    .attr("name", "value")
    ///                    .append("inner")
    ///                    .build();
    ///
    /// assert_eq!(elem.name(), "name");
    /// assert_eq!(elem.ns(), Some("namespace".to_owned()));
    /// assert_eq!(elem.attr("name"), Some("value"));
    /// assert_eq!(elem.attr("inexistent"), None);
    /// assert_eq!(elem.text(), "inner");
    /// ```
    pub fn builder<S: AsRef<str>>(name: S) -> ElementBuilder {
        let (prefix, name) = split_element_name(name).unwrap();
        ElementBuilder {
            root: Element::new(name, prefix, None, BTreeMap::new(), Vec::new()),
            namespaces: Default::default(),
        }
    }

    /// Returns a bare minimum `Element` with this name.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let bare = Element::bare("name");
    ///
    /// assert_eq!(bare.name(), "name");
    /// assert_eq!(bare.ns(), None);
    /// assert_eq!(bare.attr("name"), None);
    /// assert_eq!(bare.text(), "");
    /// ```
    pub fn bare<S: Into<String>>(name: S) -> Element {
        Element {
            prefix: None,
            name: name.into(),
            namespaces: Rc::new(NamespaceSet::default()),
            attributes: BTreeMap::new(),
            children: Vec::new(),
        }
    }

    /// Returns a reference to the name of this element.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns a reference to the prefix of this element.
    ///
    /// # Examples
    /// ```rust
    /// use minidom::Element;
    ///
    /// let elem = Element::builder("prefix:name")
    ///                    .build();
    ///
    /// assert_eq!(elem.name(), "name");
    /// assert_eq!(elem.prefix(), Some("prefix"));
    /// ```
    pub fn prefix(&self) -> Option<&str> {
        self.prefix.as_ref().map(String::as_ref)
    }

    /// Returns a reference to the namespace of this element, if it has one, else `None`.
    pub fn ns(&self) -> Option<String> {
        self.namespaces.get(&self.prefix)
    }

    /// Returns a reference to the value of the given attribute, if it exists, else `None`.
    pub fn attr(&self, name: &str) -> Option<&str> {
        if let Some(value) = self.attributes.get(name) {
            return Some(value);
        }
        None
    }

    /// Returns an iterator over the attributes of this element.
    ///
    /// # Example
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let elm: Element = "<elem a=\"b\" />".parse().unwrap();
    ///
    /// let mut iter = elm.attrs();
    ///
    /// assert_eq!(iter.next().unwrap(), ("a", "b"));
    /// assert_eq!(iter.next(), None);
    /// ```
    pub fn attrs(&self) -> Attrs {
        Attrs {
            iter: self.attributes.iter(),
        }
    }

    /// Returns an iterator over the attributes of this element, with the value being a mutable
    /// reference.
    pub fn attrs_mut(&mut self) -> AttrsMut {
        AttrsMut {
            iter: self.attributes.iter_mut(),
        }
    }

    /// Modifies the value of an attribute.
    pub fn set_attr<S: Into<String>, V: IntoAttributeValue>(&mut self, name: S, val: V) {
        let name = name.into();
        let val = val.into_attribute_value();

        if let Some(value) = self.attributes.get_mut(&name) {
            *value = val
                .expect("removing existing value via set_attr, this is not yet supported (TODO)"); // TODO
            return;
        }

        if let Some(val) = val {
            self.attributes.insert(name, val);
        }
    }

    /// Returns whether the element has the given name and namespace.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::{Element, NSChoice};
    ///
    /// let elem = Element::builder("name").ns("namespace").build();
    ///
    /// assert_eq!(elem.is("name", "namespace"), true);
    /// assert_eq!(elem.is("name", "wrong"), false);
    /// assert_eq!(elem.is("wrong", "namespace"), false);
    /// assert_eq!(elem.is("wrong", "wrong"), false);
    ///
    /// assert_eq!(elem.is("name", NSChoice::None), false);
    /// assert_eq!(elem.is("name", NSChoice::OneOf("namespace")), true);
    /// assert_eq!(elem.is("name", NSChoice::OneOf("foo")), false);
    /// assert_eq!(elem.is("name", NSChoice::AnyOf(&["foo", "namespace"])), true);
    /// assert_eq!(elem.is("name", NSChoice::Any), true);
    ///
    /// let elem2 = Element::builder("name").build();
    ///
    /// assert_eq!(elem2.is("name", NSChoice::None), true);
    /// assert_eq!(elem2.is("name", NSChoice::Any), true);
    /// ```
    pub fn is<'a, N: AsRef<str>, NS: Into<NSChoice<'a>>>(&self, name: N, namespace: NS) -> bool {
        self.name == name.as_ref() && self.has_ns(namespace)
    }

    /// Returns whether the element has the given namespace.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::{Element, NSChoice};
    ///
    /// let elem = Element::builder("name").ns("namespace").build();
    ///
    /// assert_eq!(elem.has_ns("namespace"), true);
    /// assert_eq!(elem.has_ns("wrong"), false);
    ///
    /// assert_eq!(elem.has_ns(NSChoice::None), false);
    /// assert_eq!(elem.has_ns(NSChoice::OneOf("namespace")), true);
    /// assert_eq!(elem.has_ns(NSChoice::OneOf("foo")), false);
    /// assert_eq!(elem.has_ns(NSChoice::AnyOf(&["foo", "namespace"])), true);
    /// assert_eq!(elem.has_ns(NSChoice::Any), true);
    ///
    /// let elem2 = Element::builder("name").build();
    ///
    /// assert_eq!(elem2.has_ns(NSChoice::None), true);
    /// assert_eq!(elem2.has_ns(NSChoice::Any), true);
    /// ```
    pub fn has_ns<'a, NS: Into<NSChoice<'a>>>(&self, namespace: NS) -> bool {
        self.namespaces.has(&self.prefix, namespace)
    }

    /// Parse a document from an `EventReader`.
    pub fn from_reader<R: BufRead>(reader: &mut EventReader<R>) -> Result<Element> {
        let mut buf = Vec::new();

        let root: Element = loop {
            let e = reader.read_event(&mut buf)?;
            match e {
                Event::Empty(ref e) | Event::Start(ref e) => {
                    break build_element(reader, e)?;
                }
                Event::Eof => {
                    return Err(Error::EndOfDocument);
                }
                #[cfg(not(feature = "comments"))]
                Event::Comment { .. } => {
                    return Err(Error::CommentsDisabled);
                }
                #[cfg(feature = "comments")]
                Event::Comment { .. } => (),
                Event::Text { .. }
                | Event::End { .. }
                | Event::CData { .. }
                | Event::Decl { .. }
                | Event::PI { .. }
                | Event::DocType { .. } => (), // TODO: may need more errors
            }
        };

        let mut stack = vec![root];

        loop {
            match reader.read_event(&mut buf)? {
                Event::Empty(ref e) => {
                    let elem = build_element(reader, e)?;
                    // Since there is no Event::End after, directly append it to the current node
                    stack.last_mut().unwrap().append_child(elem);
                }
                Event::Start(ref e) => {
                    let elem = build_element(reader, e)?;
                    stack.push(elem);
                }
                Event::End(ref e) => {
                    if stack.len() <= 1 {
                        break;
                    }
                    let elem = stack.pop().unwrap();
                    if let Some(to) = stack.last_mut() {
                        // TODO: check whether this is correct, we are comparing &[u8]s, not &strs
                        let elem_name = e.name();
                        let mut split_iter = elem_name.splitn(2, |u| *u == 0x3A);
                        let possible_prefix = split_iter.next().unwrap(); // Can't be empty.
                        match split_iter.next() {
                            Some(name) => {
                                match elem.prefix() {
                                    Some(prefix) => {
                                        if possible_prefix != prefix.as_bytes() {
                                            return Err(Error::InvalidElementClosed);
                                        }
                                    }
                                    None => {
                                        return Err(Error::InvalidElementClosed);
                                    }
                                }
                                if name != elem.name().as_bytes() {
                                    return Err(Error::InvalidElementClosed);
                                }
                            }
                            None => {
                                if elem.prefix().is_some() {
                                    return Err(Error::InvalidElementClosed);
                                }
                                if possible_prefix != elem.name().as_bytes() {
                                    return Err(Error::InvalidElementClosed);
                                }
                            }
                        }
                        to.append_child(elem);
                    }
                }
                Event::Text(s) => {
                    let text = s.unescape_and_decode(reader)?;
                    if text != "" {
                        let current_elem = stack.last_mut().unwrap();
                        current_elem.append_text_node(text);
                    }
                }
                Event::CData(s) => {
                    let text = reader.decode(&s)?.to_owned();
                    if text != "" {
                        let current_elem = stack.last_mut().unwrap();
                        current_elem.append_text_node(text);
                    }
                }
                Event::Eof => {
                    break;
                }
                #[cfg(not(feature = "comments"))]
                Event::Comment(_) => return Err(Error::CommentsDisabled),
                #[cfg(feature = "comments")]
                Event::Comment(s) => {
                    let comment = reader.decode(&s)?.to_owned();
                    if comment != "" {
                        let current_elem = stack.last_mut().unwrap();
                        current_elem.append_comment_node(comment);
                    }
                }
                Event::Decl { .. } | Event::PI { .. } | Event::DocType { .. } => (),
            }
        }
        Ok(stack.pop().unwrap())
    }

    /// Output a document to a `Writer`.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.to_writer(&mut EventWriter::new(writer))
    }

    /// Output a document to a `Writer`.
    pub fn write_to_decl<W: Write>(&self, writer: &mut W) -> Result<()> {
        self.to_writer_decl(&mut EventWriter::new(writer))
    }

    /// Output the document to quick-xml `Writer`
    pub fn to_writer<W: Write>(&self, writer: &mut EventWriter<W>) -> Result<()> {
        self.write_to_inner(writer)
    }

    /// Output the document to quick-xml `Writer`
    pub fn to_writer_decl<W: Write>(&self, writer: &mut EventWriter<W>) -> Result<()> {
        writer.write_event(Event::Decl(BytesDecl::new(b"1.0", Some(b"utf-8"), None)))?;
        self.write_to_inner(writer)
    }

    /// Like `write_to()` but without the `<?xml?>` prelude
    pub fn write_to_inner<W: Write>(&self, writer: &mut EventWriter<W>) -> Result<()> {
        let name = match self.prefix {
            None => Cow::Borrowed(&self.name),
            Some(ref prefix) => Cow::Owned(format!("{}:{}", prefix, self.name)),
        };

        let mut start = BytesStart::borrowed(name.as_bytes(), name.len());
        for (prefix, ns) in self.namespaces.declared_ns() {
            match *prefix {
                None => start.push_attribute(("xmlns", ns.as_ref())),
                Some(ref prefix) => {
                    let key = format!("xmlns:{}", prefix);
                    start.push_attribute((key.as_bytes(), ns.as_bytes()))
                }
            }
        }
        for (key, value) in &self.attributes {
            start.push_attribute((key.as_bytes(), escape(value.as_bytes()).as_ref()));
        }

        if self.children.is_empty() {
            writer.write_event(Event::Empty(start))?;
            return Ok(());
        }

        writer.write_event(Event::Start(start))?;

        for child in &self.children {
            child.write_to_inner(writer)?;
        }

        writer.write_event(Event::End(BytesEnd::borrowed(name.as_bytes())))?;
        Ok(())
    }

    /// Returns an iterator over references to every child node of this element.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let elem: Element = "<root>a<c1 />b<c2 />c</root>".parse().unwrap();
    ///
    /// let mut iter = elem.nodes();
    ///
    /// assert_eq!(iter.next().unwrap().as_text().unwrap(), "a");
    /// assert_eq!(iter.next().unwrap().as_element().unwrap().name(), "c1");
    /// assert_eq!(iter.next().unwrap().as_text().unwrap(), "b");
    /// assert_eq!(iter.next().unwrap().as_element().unwrap().name(), "c2");
    /// assert_eq!(iter.next().unwrap().as_text().unwrap(), "c");
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn nodes(&self) -> Nodes {
        self.children.iter()
    }

    /// Returns an iterator over mutable references to every child node of this element.
    #[inline]
    pub fn nodes_mut(&mut self) -> NodesMut {
        self.children.iter_mut()
    }

    /// Returns an iterator over references to every child element of this element.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let elem: Element = "<root>hello<child1 />this<child2 />is<child3 />ignored</root>".parse().unwrap();
    ///
    /// let mut iter = elem.children();
    /// assert_eq!(iter.next().unwrap().name(), "child1");
    /// assert_eq!(iter.next().unwrap().name(), "child2");
    /// assert_eq!(iter.next().unwrap().name(), "child3");
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn children(&self) -> Children {
        Children {
            iter: self.children.iter(),
        }
    }

    /// Returns an iterator over mutable references to every child element of this element.
    #[inline]
    pub fn children_mut(&mut self) -> ChildrenMut {
        ChildrenMut {
            iter: self.children.iter_mut(),
        }
    }

    /// Returns an iterator over references to every text node of this element.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let elem: Element = "<root>hello<c /> world!</root>".parse().unwrap();
    ///
    /// let mut iter = elem.texts();
    /// assert_eq!(iter.next().unwrap(), "hello");
    /// assert_eq!(iter.next().unwrap(), " world!");
    /// assert_eq!(iter.next(), None);
    /// ```
    #[inline]
    pub fn texts(&self) -> Texts {
        Texts {
            iter: self.children.iter(),
        }
    }

    /// Returns an iterator over mutable references to every text node of this element.
    #[inline]
    pub fn texts_mut(&mut self) -> TextsMut {
        TextsMut {
            iter: self.children.iter_mut(),
        }
    }

    /// Appends a child node to the `Element`, returning the appended node.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let mut elem = Element::bare("root");
    ///
    /// assert_eq!(elem.children().count(), 0);
    ///
    /// elem.append_child(Element::bare("child"));
    ///
    /// {
    ///     let mut iter = elem.children();
    ///     assert_eq!(iter.next().unwrap().name(), "child");
    ///     assert_eq!(iter.next(), None);
    /// }
    ///
    /// let child = elem.append_child(Element::bare("new"));
    ///
    /// assert_eq!(child.name(), "new");
    /// ```
    pub fn append_child(&mut self, child: Element) -> &mut Element {
        child.namespaces.set_parent(Rc::clone(&self.namespaces));

        self.children.push(Node::Element(child));
        if let Node::Element(ref mut cld) = *self.children.last_mut().unwrap() {
            cld
        } else {
            unreachable!()
        }
    }

    /// Appends a text node to an `Element`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let mut elem = Element::bare("node");
    ///
    /// assert_eq!(elem.text(), "");
    ///
    /// elem.append_text_node("text");
    ///
    /// assert_eq!(elem.text(), "text");
    /// ```
    pub fn append_text_node<S: Into<String>>(&mut self, child: S) {
        self.children.push(Node::Text(child.into()));
    }

    /// Appends a comment node to an `Element`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let mut elem = Element::bare("node");
    ///
    /// elem.append_comment_node("comment");
    /// ```
    #[cfg(feature = "comments")]
    pub fn append_comment_node<S: Into<String>>(&mut self, child: S) {
        self.children.push(Node::Comment(child.into()));
    }

    /// Appends a node to an `Element`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::{Element, Node};
    ///
    /// let mut elem = Element::bare("node");
    ///
    /// elem.append_node(Node::Text("hello".to_owned()));
    ///
    /// assert_eq!(elem.text(), "hello");
    /// ```
    pub fn append_node(&mut self, node: Node) {
        self.children.push(node);
    }

    /// Returns the concatenation of all text nodes in the `Element`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::Element;
    ///
    /// let elem: Element = "<node>hello,<split /> world!</node>".parse().unwrap();
    ///
    /// assert_eq!(elem.text(), "hello, world!");
    /// ```
    pub fn text(&self) -> String {
        self.texts().fold(String::new(), |ret, new| ret + new)
    }

    /// Returns a reference to the first child element with the specific name and namespace, if it
    /// exists in the direct descendants of this `Element`, else returns `None`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::{Element, NSChoice};
    ///
    /// let elem: Element = r#"<node xmlns="ns"><a /><a xmlns="other_ns" /><b /></node>"#.parse().unwrap();
    /// assert!(elem.get_child("a", "ns").unwrap().is("a", "ns"));
    /// assert!(elem.get_child("a", "other_ns").unwrap().is("a", "other_ns"));
    /// assert!(elem.get_child("b", "ns").unwrap().is("b", "ns"));
    /// assert_eq!(elem.get_child("c", "ns"), None);
    /// assert_eq!(elem.get_child("b", "other_ns"), None);
    /// assert_eq!(elem.get_child("a", "inexistent_ns"), None);
    ///
    /// let elem: Element = r#"<node><a xmlns="other_ns" /><b /></node>"#.parse().unwrap();
    /// assert_eq!(elem.get_child("a", NSChoice::None), None);
    /// assert!(elem.get_child("a", NSChoice::Any).unwrap().is("a", "other_ns"));
    /// assert!(elem.get_child("b", NSChoice::None).unwrap().is("b", NSChoice::None));
    /// assert!(elem.get_child("b", NSChoice::Any).unwrap().is("b", NSChoice::None));
    /// ```
    pub fn get_child<'a, N: AsRef<str>, NS: Into<NSChoice<'a>>>(
        &self,
        name: N,
        namespace: NS,
    ) -> Option<&Element> {
        let namespace = namespace.into();
        for fork in &self.children {
            if let Node::Element(ref e) = *fork {
                if e.is(name.as_ref(), namespace) {
                    return Some(e);
                }
            }
        }
        None
    }

    /// Returns a mutable reference to the first child element with the specific name and namespace,
    /// if it exists in the direct descendants of this `Element`, else returns `None`.
    pub fn get_child_mut<'a, N: AsRef<str>, NS: Into<NSChoice<'a>>>(
        &mut self,
        name: N,
        namespace: NS,
    ) -> Option<&mut Element> {
        let namespace = namespace.into();
        for fork in &mut self.children {
            if let Node::Element(ref mut e) = *fork {
                if e.is(name.as_ref(), namespace) {
                    return Some(e);
                }
            }
        }
        None
    }

    /// Returns whether a specific child with this name and namespace exists in the direct
    /// descendants of the `Element`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::{Element, NSChoice};
    ///
    /// let elem: Element = r#"<node xmlns="ns"><a /><a xmlns="other_ns" /><b /></node>"#.parse().unwrap();
    /// assert_eq!(elem.has_child("a", "other_ns"), true);
    /// assert_eq!(elem.has_child("a", "ns"), true);
    /// assert_eq!(elem.has_child("a", "inexistent_ns"), false);
    /// assert_eq!(elem.has_child("b", "ns"), true);
    /// assert_eq!(elem.has_child("b", "other_ns"), false);
    /// assert_eq!(elem.has_child("b", "inexistent_ns"), false);
    ///
    /// let elem: Element = r#"<node><a xmlns="other_ns" /><b /></node>"#.parse().unwrap();
    /// assert_eq!(elem.has_child("a", NSChoice::None), false);
    /// assert_eq!(elem.has_child("a", NSChoice::OneOf("other_ns")), true);
    /// assert_eq!(elem.has_child("a", NSChoice::Any), true);
    /// assert_eq!(elem.has_child("b", NSChoice::None), true);
    /// ```
    pub fn has_child<'a, N: AsRef<str>, NS: Into<NSChoice<'a>>>(
        &self,
        name: N,
        namespace: NS,
    ) -> bool {
        self.get_child(name, namespace).is_some()
    }

    /// Removes the first child with this name and namespace, if it exists, and returns an
    /// `Option<Element>` containing this child if it succeeds.
    /// Returns `None` if no child matches this name and namespace.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use minidom::{Element, NSChoice};
    ///
    /// let mut elem: Element = r#"<node xmlns="ns"><a /><a xmlns="other_ns" /><b /></node>"#.parse().unwrap();
    /// assert!(elem.remove_child("a", "ns").unwrap().is("a", "ns"));
    /// assert!(elem.remove_child("a", "ns").is_none());
    /// assert!(elem.remove_child("inexistent", "inexistent").is_none());
    ///
    /// let mut elem: Element = r#"<node><a xmlns="other_ns" /><b /></node>"#.parse().unwrap();
    /// assert!(elem.remove_child("a", NSChoice::None).is_none());
    /// assert!(elem.remove_child("a", NSChoice::Any).unwrap().is("a", "other_ns"));
    /// assert!(elem.remove_child("b", NSChoice::None).unwrap().is("b", NSChoice::None));
    /// ```
    pub fn remove_child<'a, N: AsRef<str>, NS: Into<NSChoice<'a>>>(
        &mut self,
        name: N,
        namespace: NS,
    ) -> Option<Element> {
        let name = name.as_ref();
        let namespace = namespace.into();
        let idx = self.children.iter().position(|x| {
            if let Node::Element(ref elm) = x {
                elm.is(name, namespace)
            } else {
                false
            }
        })?;
        self.children.remove(idx).into_element()
    }
}

fn split_element_name<S: AsRef<str>>(s: S) -> Result<(Option<String>, String)> {
    let name_parts = s.as_ref().split(':').collect::<Vec<&str>>();
    match name_parts.len() {
        2 => Ok((Some(name_parts[0].to_owned()), name_parts[1].to_owned())),
        1 => Ok((None, name_parts[0].to_owned())),
        _ => Err(Error::InvalidElement),
    }
}

fn build_element<R: BufRead>(reader: &EventReader<R>, event: &BytesStart) -> Result<Element> {
    let mut namespaces = BTreeMap::new();
    let attributes = event
        .attributes()
        .map(|o| {
            let o = o?;
            let key = str::from_utf8(o.key)?.to_owned();
            let value = o.unescape_and_decode_value(reader)?;
            Ok((key, value))
        })
        .filter(|o| match *o {
            Ok((ref key, ref value)) if key == "xmlns" => {
                namespaces.insert(None, value.to_owned());
                false
            }
            Ok((ref key, ref value)) if key.starts_with("xmlns:") => {
                namespaces.insert(Some(key[6..].to_owned()), value.to_owned());
                false
            }
            _ => true,
        })
        .collect::<Result<BTreeMap<String, String>>>()?;

    let (prefix, name) = split_element_name(str::from_utf8(event.name())?)?;
    let element = Element::new(name, prefix, namespaces, attributes, Vec::new());
    Ok(element)
}

/// An iterator over references to child elements of an `Element`.
pub struct Children<'a> {
    iter: slice::Iter<'a, Node>,
}

impl<'a> Iterator for Children<'a> {
    type Item = &'a Element;

    fn next(&mut self) -> Option<&'a Element> {
        for item in &mut self.iter {
            if let Node::Element(ref child) = *item {
                return Some(child);
            }
        }
        None
    }
}

/// An iterator over mutable references to child elements of an `Element`.
pub struct ChildrenMut<'a> {
    iter: slice::IterMut<'a, Node>,
}

impl<'a> Iterator for ChildrenMut<'a> {
    type Item = &'a mut Element;

    fn next(&mut self) -> Option<&'a mut Element> {
        for item in &mut self.iter {
            if let Node::Element(ref mut child) = *item {
                return Some(child);
            }
        }
        None
    }
}

/// An iterator over references to child text nodes of an `Element`.
pub struct Texts<'a> {
    iter: slice::Iter<'a, Node>,
}

impl<'a> Iterator for Texts<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        for item in &mut self.iter {
            if let Node::Text(ref child) = *item {
                return Some(child);
            }
        }
        None
    }
}

/// An iterator over mutable references to child text nodes of an `Element`.
pub struct TextsMut<'a> {
    iter: slice::IterMut<'a, Node>,
}

impl<'a> Iterator for TextsMut<'a> {
    type Item = &'a mut String;

    fn next(&mut self) -> Option<&'a mut String> {
        for item in &mut self.iter {
            if let Node::Text(ref mut child) = *item {
                return Some(child);
            }
        }
        None
    }
}

/// An iterator over references to all child nodes of an `Element`.
pub type Nodes<'a> = slice::Iter<'a, Node>;

/// An iterator over mutable references to all child nodes of an `Element`.
pub type NodesMut<'a> = slice::IterMut<'a, Node>;

/// An iterator over the attributes of an `Element`.
pub struct Attrs<'a> {
    iter: btree_map::Iter<'a, String, String>,
}

impl<'a> Iterator for Attrs<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(x, y)| (x.as_ref(), y.as_ref()))
    }
}

/// An iterator over the attributes of an `Element`, with the values mutable.
pub struct AttrsMut<'a> {
    iter: btree_map::IterMut<'a, String, String>,
}

impl<'a> Iterator for AttrsMut<'a> {
    type Item = (&'a str, &'a mut String);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(x, y)| (x.as_ref(), y))
    }
}

/// A builder for `Element`s.
pub struct ElementBuilder {
    root: Element,
    namespaces: BTreeMap<Option<String>, String>,
}

impl ElementBuilder {
    /// Sets the namespace.
    pub fn ns<S: Into<String>>(mut self, namespace: S) -> ElementBuilder {
        self.namespaces
            .insert(self.root.prefix.clone(), namespace.into());
        self
    }

    /// Sets an attribute.
    pub fn attr<S: Into<String>, V: IntoAttributeValue>(
        mut self,
        name: S,
        value: V,
    ) -> ElementBuilder {
        self.root.set_attr(name, value);
        self
    }

    /// Appends anything implementing `Into<Node>` into the tree.
    pub fn append<T: Into<Node>>(mut self, node: T) -> ElementBuilder {
        self.root.append_node(node.into());
        self
    }

    /// Appends an iterator of things implementing `Into<Node>` into the tree.
    pub fn append_all<T: Into<Node>, I: IntoIterator<Item = T>>(
        mut self,
        iter: I,
    ) -> ElementBuilder {
        for node in iter {
            self.root.append_node(node.into());
        }
        self
    }

    /// Builds the `Element`.
    pub fn build(self) -> Element {
        let mut element = self.root;
        // Set namespaces
        element.namespaces = Rc::new(NamespaceSet::from(self.namespaces));
        // Propagate namespaces
        for node in &element.children {
            if let Node::Element(ref e) = *node {
                e.namespaces.set_parent(Rc::clone(&element.namespaces));
            }
        }
        element
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_element_new() {
        use std::iter::FromIterator;

        let elem = Element::new(
            "name".to_owned(),
            None,
            Some("namespace".to_owned()),
            BTreeMap::from_iter(vec![("name".to_string(), "value".to_string())].into_iter()),
            Vec::new(),
        );

        assert_eq!(elem.name(), "name");
        assert_eq!(elem.ns(), Some("namespace".to_owned()));
        assert_eq!(elem.attr("name"), Some("value"));
        assert_eq!(elem.attr("inexistent"), None);
    }

    #[test]
    fn test_from_reader_simple() {
        let xml = "<foo></foo>";
        let mut reader = EventReader::from_str(xml);
        let elem = Element::from_reader(&mut reader);

        let elem2 = Element::builder("foo").build();

        assert_eq!(elem.unwrap(), elem2);
    }

    #[test]
    fn test_from_reader_nested() {
        let xml = "<foo><bar baz='qxx' /></foo>";
        let mut reader = EventReader::from_str(xml);
        let elem = Element::from_reader(&mut reader);

        let nested = Element::builder("bar").attr("baz", "qxx").build();
        let elem2 = Element::builder("foo").append(nested).build();

        assert_eq!(elem.unwrap(), elem2);
    }

    #[test]
    fn test_from_reader_with_prefix() {
        let xml = "<foo><prefix:bar baz='qxx' /></foo>";
        let mut reader = EventReader::from_str(xml);
        let elem = Element::from_reader(&mut reader);

        let nested = Element::builder("prefix:bar").attr("baz", "qxx").build();
        let elem2 = Element::builder("foo").append(nested).build();

        assert_eq!(elem.unwrap(), elem2);
    }

    #[test]
    fn parses_spectest_xml() {
        // From: https://gitlab.com/lumi/minidom-rs/issues/8
        let xml = r#"
            <rng:grammar xmlns:rng="http://relaxng.org/ns/structure/1.0">
                <rng:name xmlns:rng="http://relaxng.org/ns/structure/1.0"></rng:name>
            </rng:grammar>
        "#;
        let mut reader = EventReader::from_str(xml);
        let _ = Element::from_reader(&mut reader).unwrap();
    }

    #[test]
    fn does_not_unescape_cdata() {
        let xml = "<test><![CDATA[&apos;&gt;blah<blah>]]></test>";
        let mut reader = EventReader::from_str(xml);
        let elem = Element::from_reader(&mut reader).unwrap();
        assert_eq!(elem.text(), "&apos;&gt;blah<blah>");
    }

    #[test]
    fn test_compare_all_ns() {
        let xml = "<foo xmlns='foo' xmlns:bar='baz'><bar:meh/></foo>";
        let mut reader = EventReader::from_str(xml);
        let elem = Element::from_reader(&mut reader).unwrap();

        let elem2 = elem.clone();

        let xml3 = "<foo xmlns='foo'><bar:meh xmlns:bar='baz'/></foo>";
        let mut reader3 = EventReader::from_str(xml3);
        let elem3 = Element::from_reader(&mut reader3).unwrap();

        let xml4 = "<prefix:foo xmlns:prefix='foo'><bar:meh xmlns:bar='baz'/></prefix:foo>";
        let mut reader4 = EventReader::from_str(xml4);
        let elem4 = Element::from_reader(&mut reader4).unwrap();

        assert_eq!(elem, elem2);
        assert_eq!(elem, elem3);
        assert_eq!(elem, elem4);
    }
}
