/*******************************************************************************
 * Copyright (c) 2020 Association Cénotélie (cenotelie.fr)
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Lesser General Public License as
 * published by the Free Software Foundation, either version 3
 * of the License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Lesser General Public License for more details.
 *
 * You should have received a copy of the GNU Lesser General
 * Public License along with this program.
 * If not, see <http://www.gnu.org/licenses/>.
 ******************************************************************************/

//! Module for the definition of a server-side workspace

use hime_sdk::errors::Error;
use hime_sdk::{CompilationTask, Input, InputReference, LoadedData};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, ErrorKind, Read};
use std::path::PathBuf;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, FileChangeType, FileEvent,
    Position, Range, Url
};

/// Represents a document in a workspace
#[derive(Debug, Clone)]
pub struct Document {
    /// The content of the document in this version
    pub content: String,
    /// The current version
    pub version: Option<i64>,
    /// The diagnostics for the document
    pub diagnostics: Vec<Diagnostic>
}

impl Document {
    /// Creates a new document
    pub fn new(content: String) -> Document {
        Document {
            content: content,
            version: None,
            diagnostics: Vec::new()
        }
    }
}

/// Represents the current workspace for a server
#[derive(Debug, Clone, Default)]
pub struct Workspace {
    /// The root URL for the workspace
    pub root: Option<Url>,
    /// The documents in the workspace
    pub documents: HashMap<Url, Document>
}

impl Workspace {
    /// Scans the current workspace for relevant documents
    pub fn scan_workspace(&mut self, root: Url) -> io::Result<()> {
        let path = PathBuf::from(root.path());
        if path.exists() {
            self.scan_workspace_in(&path)?;
        }
        self.root = Some(root);
        Ok(())
    }

    /// Scans the workspace in the specified folder
    fn scan_workspace_in(&mut self, path: &PathBuf) -> io::Result<()> {
        if Workspace::scan_workspace_is_dir_excluded(path) {
            return Ok(());
        }
        for element in std::fs::read_dir(path)? {
            let sub_path = element?.path();
            if sub_path.is_dir() {
                self.scan_workspace_in(&sub_path)?;
            } else if Workspace::scan_workspace_is_file_included(&sub_path) {
                self.resolve_document_path(&sub_path)?;
            }
        }
        Ok(())
    }

    /// Determines whether the specified file should be analyzed
    fn scan_workspace_is_file_included(path: &PathBuf) -> bool {
        match path.extension() {
            None => false,
            Some(name) => name == "gram"
        }
    }

    /// Determines whether the specified file or directory is excluded
    fn scan_workspace_is_dir_excluded(path: &PathBuf) -> bool {
        match path.file_name() {
            None => true,
            Some(name) => name == ".git" || name == ".hg" || name == ".svn"
        }
    }

    /// Resolves a document
    fn resolve_document_path(&mut self, path: &PathBuf) -> io::Result<()> {
        let uri = match Url::from_file_path(path.canonicalize()?) {
            Ok(uri) => uri,
            Err(_) => {
                return Err(io::Error::new(
                    ErrorKind::NotFound,
                    String::from("Path cannot be converted to Url")
                ))
            }
        };
        self.resolve_document(uri, path)
    }

    /// Resolves a document
    fn resolve_document_url(&mut self, uri: Url) -> io::Result<()> {
        let path = PathBuf::from(uri.path());
        self.resolve_document(uri, &path)
    }

    /// Resolves a document
    fn resolve_document(&mut self, uri: Url, path: &PathBuf) -> io::Result<()> {
        let mut reader = BufReader::new(File::open(path)?);
        let mut content = String::new();
        reader.read_to_string(&mut content)?;
        self.documents
            .entry(uri)
            .or_insert_with(|| Document::new(content));
        Ok(())
    }

    /// Synchronises on file events
    pub fn on_file_events(&mut self, events: &[FileEvent]) -> io::Result<()> {
        for event in events.iter() {
            match event.typ {
                FileChangeType::Created => {
                    self.resolve_document_url(event.uri.clone())?;
                }
                FileChangeType::Changed => {
                    // TODO: handle this
                }
                FileChangeType::Deleted => {
                    self.documents.remove(&event.uri);
                }
            }
        }
        Ok(())
    }

    /// Synchronizes on changes
    pub fn on_file_changes(&mut self, event: DidChangeTextDocumentParams) {
        if let Some(document) = self.documents.get_mut(&event.text_document.uri) {
            for change in event.content_changes.into_iter() {
                if change.range.is_none() && change.range_length.is_none() {
                    document.content = change.text;
                }
            }
        }
    }

    /// Runs the diagnostics
    pub fn lint(&mut self) {
        let mut task = CompilationTask::default();
        let mut documents: Vec<&mut Document> =
            self.documents.iter_mut().map(|(_, doc)| doc).collect();
        for doc in documents.iter_mut() {
            task.inputs.push(Input::Raw(&doc.content));
            doc.diagnostics.clear();
        }
        match task.load() {
            Ok(mut data) => {
                let mut errors = Vec::new();
                for (index, grammar) in data.grammars.iter_mut().enumerate() {
                    if let Err(mut errs) = task.generate_in_memory(grammar, index) {
                        errors.append(&mut errs);
                    }
                }
                for error in errors.iter() {
                    if let Some((index, diag)) = to_diagnostic(&data, error) {
                        documents[index].diagnostics.push(diag);
                    }
                }
            }
            Err(errors) => {
                for error in errors.errors.iter() {
                    if let Some((index, diag)) = to_diagnostic(&errors.data, error) {
                        documents[index].diagnostics.push(diag);
                    }
                }
            }
        }
    }
}

/// Converts an error to a diagnostic
fn to_diagnostic(data: &LoadedData, error: &Error) -> Option<(usize, Diagnostic)> {
    match error {
        Error::Parsing(input_reference, msg) => Some((
            input_reference.input_index,
            Diagnostic {
                range: to_range(data, *input_reference),
                severity: Some(DiagnosticSeverity::Error),
                code: None,
                source: Some(super::CRATE_NAME.to_string()),
                message: msg.clone(),
                related_information: None,
                tags: None
            }
        )),
        _ => None
    }
}

/// Translate an input reference to a LSP range
fn to_range(data: &LoadedData, input_reference: InputReference) -> Range {
    let end = data.inputs[input_reference.input_index]
        .content
        .get_position_for(input_reference.position, input_reference.length);
    Range::new(
        Position::new(
            (input_reference.position.line - 1) as u64,
            (input_reference.position.column - 1) as u64
        ),
        Position::new((end.line - 1) as u64, (end.column - 1) as u64)
    )
}

#[test]
fn test_scan_workspace_in() -> io::Result<()> {
    let mut workspace = Workspace::default();
    let root = std::env::current_dir()?.parent().unwrap().to_owned();
    workspace.scan_workspace_in(&root)?;
    for (uri, _) in workspace.documents.iter() {
        println!("{}", uri);
    }
    assert_eq!(workspace.documents.is_empty(), false);
    Ok(())
}

#[test]
fn test_scan_workspace() -> io::Result<()> {
    let mut workspace = Workspace::default();
    let root = std::env::current_dir()?.parent().unwrap().to_owned();
    let url = match Url::from_file_path(root) {
        Ok(url) => url,
        Err(_) => panic!("Failed to convert current dir to Url")
    };
    workspace.scan_workspace(url)?;
    for (uri, _) in workspace.documents.iter() {
        println!("{}", uri);
    }
    assert_eq!(workspace.documents.is_empty(), false);
    Ok(())
}
