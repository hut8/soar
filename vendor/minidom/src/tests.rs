// Copyright (c) 2020 lumi <lumi@pew.im>
// Copyright (c) 2020 Emmanuel Gil Peyrot <linkmauve@linkmauve.fr>
// Copyright (c) 2020 Bastien Orivel <eijebong+minidom@bananium.fr>
// Copyright (c) 2020 Astro <astro@spaceboyz.net>
// Copyright (c) 2020 Maxime “pep” Buquet <pep@bouah.net>
// Copyright (c) 2020 Yue Liu <amznyue@amazon.com>
// Copyright (c) 2020 Matt Bilker <me@mbilker.us>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::element::Element;

use quick_xml::Reader;

const TEST_STRING: &'static str = r#"<root xmlns="root_ns" a="b" xml:lang="en">meow<child c="d"/><child xmlns="child_ns" d="e" xml:lang="fr"/>nya</root>"#;

fn build_test_tree() -> Element {
    let mut root = Element::builder("root")
        .ns("root_ns")
        .attr("xml:lang", "en")
        .attr("a", "b")
        .build();
    root.append_text_node("meow");
    let child = Element::builder("child").attr("c", "d").build();
    root.append_child(child);
    let other_child = Element::builder("child")
        .ns("child_ns")
        .attr("d", "e")
        .attr("xml:lang", "fr")
        .build();
    root.append_child(other_child);
    root.append_text_node("nya");
    root
}

#[cfg(feature = "comments")]
const COMMENT_TEST_STRING: &'static str = r#"<root><!--This is a child.--><child attr="val"><!--This is a grandchild.--><grandchild/></child></root>"#;

#[cfg(feature = "comments")]
fn build_comment_test_tree() -> Element {
    let mut root = Element::builder("root").build();
    root.append_comment_node("This is a child.");
    let mut child = Element::builder("child").attr("attr", "val").build();
    child.append_comment_node("This is a grandchild.");
    let grand_child = Element::builder("grandchild").build();
    child.append_child(grand_child);
    root.append_child(child);
    root
}

#[test]
fn reader_works() {
    let mut reader = Reader::from_str(TEST_STRING);
    assert_eq!(
        Element::from_reader(&mut reader).unwrap(),
        build_test_tree()
    );
}

#[test]
fn writer_works() {
    let root = build_test_tree();
    let mut writer = Vec::new();
    {
        root.write_to(&mut writer).unwrap();
    }
    assert_eq!(String::from_utf8(writer).unwrap(), TEST_STRING);
}

#[test]
fn writer_with_decl_works() {
    let root = build_test_tree();
    let mut writer = Vec::new();
    {
        root.write_to_decl(&mut writer).unwrap();
    }
    let result = format!(r#"<?xml version="1.0" encoding="utf-8"?>{}"#, TEST_STRING);
    assert_eq!(String::from_utf8(writer).unwrap(), result);
}

#[test]
fn writer_escapes_attributes() {
    let root = Element::builder("root").attr("a", "\"Air\" quotes").build();
    let mut writer = Vec::new();
    {
        root.write_to(&mut writer).unwrap();
    }
    assert_eq!(
        String::from_utf8(writer).unwrap(),
        r#"<root a="&quot;Air&quot; quotes"/>"#
    );
}

#[test]
fn writer_escapes_text() {
    let root = Element::builder("root").append("<3").build();
    let mut writer = Vec::new();
    {
        root.write_to(&mut writer).unwrap();
    }
    assert_eq!(String::from_utf8(writer).unwrap(), r#"<root>&lt;3</root>"#);
}

#[test]
fn builder_works() {
    let elem = Element::builder("a")
        .ns("b")
        .attr("c", "d")
        .append(Element::builder("child"))
        .append("e")
        .build();
    assert_eq!(elem.name(), "a");
    assert_eq!(elem.ns(), Some("b".to_owned()));
    assert_eq!(elem.attr("c"), Some("d"));
    assert_eq!(elem.attr("x"), None);
    assert_eq!(elem.text(), "e");
    assert!(elem.has_child("child", "b"));
    assert!(elem.is("a", "b"));
}

#[test]
fn children_iter_works() {
    let root = build_test_tree();
    let mut iter = root.children();
    assert!(iter.next().unwrap().is("child", "root_ns"));
    assert!(iter.next().unwrap().is("child", "child_ns"));
    assert_eq!(iter.next(), None);
}

#[test]
fn get_child_works() {
    let root = build_test_tree();
    assert_eq!(root.get_child("child", "inexistent_ns"), None);
    assert_eq!(root.get_child("not_a_child", "root_ns"), None);
    assert!(root
        .get_child("child", "root_ns")
        .unwrap()
        .is("child", "root_ns"));
    assert!(root
        .get_child("child", "child_ns")
        .unwrap()
        .is("child", "child_ns"));
    assert_eq!(
        root.get_child("child", "root_ns").unwrap().attr("c"),
        Some("d")
    );
    assert_eq!(
        root.get_child("child", "child_ns").unwrap().attr("d"),
        Some("e")
    );
}

#[test]
fn namespace_propagation_works() {
    let mut root = Element::builder("root").ns("root_ns").build();
    let mut child = Element::bare("child");
    let grandchild = Element::bare("grandchild");
    child.append_child(grandchild);
    root.append_child(child);

    assert_eq!(root.get_child("child", "root_ns").unwrap().ns(), root.ns());
    assert_eq!(
        root.get_child("child", "root_ns")
            .unwrap()
            .get_child("grandchild", "root_ns")
            .unwrap()
            .ns(),
        root.ns()
    );
}

#[test]
fn two_elements_with_same_arguments_different_order_are_equal() {
    let elem1: Element = "<a b='a' c=''/>".parse().unwrap();
    let elem2: Element = "<a c='' b='a'/>".parse().unwrap();
    assert_eq!(elem1, elem2);

    let elem1: Element = "<a b='a' c=''/>".parse().unwrap();
    let elem2: Element = "<a c='d' b='a'/>".parse().unwrap();
    assert_ne!(elem1, elem2);
}

#[test]
fn namespace_attributes_works() {
    let mut reader = Reader::from_str(TEST_STRING);
    let root = Element::from_reader(&mut reader).unwrap();
    assert_eq!("en", root.attr("xml:lang").unwrap());
    assert_eq!(
        "fr",
        root.get_child("child", "child_ns")
            .unwrap()
            .attr("xml:lang")
            .unwrap()
    );
}

#[test]
fn wrongly_closed_elements_error() {
    let elem1 = "<a></b>".parse::<Element>();
    assert!(elem1.is_err());
    let elem1 = "<a></c></a>".parse::<Element>();
    assert!(elem1.is_err());
    let elem1 = "<a><c><d/></c></a>".parse::<Element>();
    assert!(elem1.is_ok());
}

#[test]
fn namespace_simple() {
    let elem: Element = "<message xmlns='jabber:client'/>".parse().unwrap();
    assert_eq!(elem.name(), "message");
    assert_eq!(elem.ns(), Some("jabber:client".to_owned()));
}

#[test]
fn namespace_prefixed() {
    let elem: Element = "<stream:features xmlns:stream='http://etherx.jabber.org/streams'/>"
        .parse()
        .unwrap();
    assert_eq!(elem.name(), "features");
    assert_eq!(
        elem.ns(),
        Some("http://etherx.jabber.org/streams".to_owned())
    );
}

#[test]
fn namespace_inherited_simple() {
    let elem: Element = "<stream xmlns='jabber:client'><message/></stream>"
        .parse()
        .unwrap();
    assert_eq!(elem.name(), "stream");
    assert_eq!(elem.ns(), Some("jabber:client".to_owned()));
    let child = elem.children().next().unwrap();
    assert_eq!(child.name(), "message");
    assert_eq!(child.ns(), Some("jabber:client".to_owned()));
}

#[test]
fn namespace_inherited_prefixed1() {
    let elem: Element = "<stream:features xmlns:stream='http://etherx.jabber.org/streams' xmlns='jabber:client'><message/></stream:features>"
        .parse().unwrap();
    assert_eq!(elem.name(), "features");
    assert_eq!(
        elem.ns(),
        Some("http://etherx.jabber.org/streams".to_owned())
    );
    let child = elem.children().next().unwrap();
    assert_eq!(child.name(), "message");
    assert_eq!(child.ns(), Some("jabber:client".to_owned()));
}

#[test]
fn namespace_inherited_prefixed2() {
    let elem: Element = "<stream xmlns='http://etherx.jabber.org/streams' xmlns:jabber='jabber:client'><jabber:message/></stream>"
        .parse().unwrap();
    assert_eq!(elem.name(), "stream");
    assert_eq!(
        elem.ns(),
        Some("http://etherx.jabber.org/streams".to_owned())
    );
    let child = elem.children().next().unwrap();
    assert_eq!(child.name(), "message");
    assert_eq!(child.ns(), Some("jabber:client".to_owned()));
}

#[cfg(feature = "comments")]
#[test]
fn read_comments() {
    let mut reader = Reader::from_str(COMMENT_TEST_STRING);
    assert_eq!(
        Element::from_reader(&mut reader).unwrap(),
        build_comment_test_tree()
    );
}

#[cfg(feature = "comments")]
#[test]
fn write_comments() {
    let root = build_comment_test_tree();
    let mut writer = Vec::new();
    {
        root.write_to(&mut writer).unwrap();
    }
    assert_eq!(String::from_utf8(writer).unwrap(), COMMENT_TEST_STRING);
}

#[test]
fn xml_error() {
    match "<a></b>".parse::<Element>() {
        Err(crate::error::Error::XmlError(_)) => (),
        err => panic!("No or wrong error: {:?}", err),
    }

    match "<a></".parse::<Element>() {
        Err(crate::error::Error::XmlError(_)) => (),
        err => panic!("No or wrong error: {:?}", err),
    }
}

#[test]
fn invalid_element_error() {
    match "<a:b:c>".parse::<Element>() {
        Err(crate::error::Error::InvalidElement) => (),
        err => panic!("No or wrong error: {:?}", err),
    }
}
