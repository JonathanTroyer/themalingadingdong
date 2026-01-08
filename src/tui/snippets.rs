//! Code snippets for syntax highlighting preview.

pub struct CodeSnippet {
    pub extension: &'static str,
    pub display_name: &'static str,
    pub code: &'static str,
}

pub const SNIPPETS: &[CodeSnippet] = &[
    CodeSnippet {
        extension: "rs",
        display_name: "Rust",
        code: include_str!("snippets/sample.rs"),
    },
    CodeSnippet {
        extension: "py",
        display_name: "Python",
        code: include_str!("snippets/sample.py"),
    },
    CodeSnippet {
        extension: "js",
        display_name: "JavaScript",
        code: include_str!("snippets/sample.js"),
    },
    CodeSnippet {
        extension: "go",
        display_name: "Go",
        code: include_str!("snippets/sample.go"),
    },
    CodeSnippet {
        extension: "sh",
        display_name: "Shell",
        code: include_str!("snippets/sample.sh"),
    },
];
