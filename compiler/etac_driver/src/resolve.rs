//! Path resolution: the single owner of "which file does this name mean".
//!
//! Everything that turns a user-facing name into a [`FileId`] lives here:
//!
//! * command-line paths → classified [`File`]s, with relative paths resolved
//!   against `--sourcepath`;
//! * `use F` declarations → `F.eti` (spec section 8), searched next to the
//!   using source file first and then under `--libpath`;
//! * deduplication across *all* entry points, so a file reached twice — two
//!   `use`s, a `use` plus a command-line mention, a repeated argument — is
//!   compiled exactly once. The spec explicitly permits referencing the same
//!   interface more than once; without the shared seen-set that meant parsing
//!   it (and reporting its errors) more than once.
//!
//! The driver never touches [`Path`] logic directly; it hands names in and
//! gets ids out. Failures are reported here too (through the [`DiagCtxt`]), so
//! callers only ever see `Option`: `None` always means "skip, and the user
//! already knows why" — either the failure was just reported, or the file is
//! already queued and skipping is the correct, silent thing to do.
//!
//! Interfaces cannot themselves contain `use` declarations (the grammar gives
//! interfaces no use-list), so resolution is a flat pass rather than a
//! transitive worklist. If interfaces ever grow uses, this module is where the
//! worklist goes.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use etac_errors::{DiagCtxt, etac_error};
use etac_span::{FileId, InterfaceId, SourceId, Span};

/// A classified command-line input.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum File {
    Program(SourceId),
    Interface(InterfaceId),
}

/// See the module docs. Construct one per compilation ([`Resolver::new`]) and
/// route every name through it — the shared seen-set is what makes the
/// exactly-once guarantee hold across entry points.
pub struct Resolver {
    source_path: PathBuf,
    lib_path: PathBuf,
    /// Every [`FileId`] handed out so far, keyed by resolved path.
    seen: HashSet<FileId>,
}

impl Resolver {
    #[must_use]
    pub fn new(source_path: &Path, lib_path: &Path) -> Self {
        Self {
            source_path: source_path.to_path_buf(),
            lib_path: lib_path.to_path_buf(),
            seen: HashSet::new(),
        }
    }

    /// Classify a file named on the command line, resolving relative paths
    /// against `--sourcepath`. Existence is *not* checked here — that stays a
    /// load-time concern, where the I/O error carries the real cause.
    ///
    /// `None` means skip: the path was unusable (non-UTF8 name or unknown
    /// extension; reported) or names a file that is already queued (silent).
    pub fn classify_cli(&mut self, dcx: &DiagCtxt, path: &Path) -> Option<File> {
        let path = resolve_against(&self.source_path, path);
        let Some(path_str) = path.to_str() else {
            dcx.err_no_span(format!("non-UTF8 file name {}", path.to_string_lossy()))
                .emit();
            return None;
        };

        let file = match path.extension().and_then(|x| x.to_str()) {
            Some("eta") => File::Program(SourceId::new(path_str)),
            Some("eti") => File::Interface(InterfaceId::new(path_str)),
            ext => {
                dcx.err_no_span(format!(
                    "unknown file type `{}` for {path_str}",
                    ext.unwrap_or("")
                ))
                .emit();
                return None;
            }
        };
        let (File::Program(id) | File::Interface(id)) = file;
        self.seen.insert(id).then_some(file)
    }

    /// Resolve one `use name` appearing in `from`. The search order is the directory
    /// of `from`, then `--libpath`.
    ///
    /// Takes the name and blame span rather than an AST node, so the resolver
    /// stays independent of `etac_ast` and trivially testable.
    ///
    /// `Err` means skip: no candidate exists on the search path (reported at
    /// `at`, naming every location searched) 
    /// `None` means the interface is already queued. 
    pub fn resolve_use(
        &mut self,
        dcx: &DiagCtxt,
        from: SourceId,
        name: &str,
        at: Span,
    ) -> super::Result<Option<InterfaceId>> {
        let file_name = format!("{name}.eti");
        let from_dir = Path::new(from.as_str())
            .parent()
            .unwrap_or_else(|| Path::new(""));

        let mut candidates = vec![from_dir.join(&file_name)];
        let in_lib = self.lib_path.join(&file_name);
        if in_lib != candidates[0] {
            candidates.push(in_lib);
        }

        for candidate in &candidates {
            if candidate.is_file() {
                let iid = InterfaceId::new(candidate.to_string_lossy());
                return Ok(self.seen.insert(iid).then_some(iid));
            }
        }

        let searched = candidates
            .iter()
            .map(|c| c.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        Err(etac_error! {
            dcx, at, "cannot find interface `{}`", name;
            primary: "no `{}` on the search path", file_name;
            note: "searched: {}", searched;
        }
        .emit())
    }
}

/// Join `path` onto `root` unless the path is absolute or the root is the
/// default `.` — keeping the common no-flag case byte-identical to what the
/// user typed, so diagnostics and `-D` log paths reproduce it verbatim.
fn resolve_against(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() || root == Path::new(".") {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use etac_errors::{BufferEmitter, Level, RecordedDiag};

    /// Run `f` with a context whose diagnostics are captured instead of
    /// printed, returning whatever it produced plus the recorded diagnostics.
    fn with_dcx<T>(f: impl FnOnce(&DiagCtxt) -> T) -> (T, Vec<RecordedDiag>) {
        let buf = BufferEmitter::new();
        let out = {
            let dcx = DiagCtxt::with_emitter(etac_span::sources(), Box::new(buf.clone()));
            f(&dcx)
        };
        (out, buf.take())
    }

    fn error_count(diags: &[RecordedDiag]) -> usize {
        diags.iter().filter(|d| d.level == Level::Error).count()
    }

    #[test]
    fn sourcepath_prefixes_relative_cli_paths() {
        let root = tempfile::tempdir().unwrap();
        let mut r = Resolver::new(root.path(), Path::new("."));
        let (file, diags) = with_dcx(|dcx| r.classify_cli(dcx, Path::new("sub/foo.eta")));
        let Some(File::Program(id)) = file else {
            panic!("expected a program, got {file:?}")
        };
        assert_eq!(id.as_str(), root.path().join("sub/foo.eta").to_str().unwrap());
        assert_eq!(error_count(&diags), 0);
    }

    #[test]
    fn default_sourcepath_and_absolute_paths_stay_verbatim() {
        let mut r = Resolver::new(Path::new("."), Path::new("."));
        let (file, _) = with_dcx(|dcx| r.classify_cli(dcx, Path::new("sub/foo.eta")));
        let Some(File::Program(id)) = file else { panic!() };
        assert_eq!(id.as_str(), "sub/foo.eta", "default `.` must not rewrite the path");

        let root = tempfile::tempdir().unwrap();
        let mut r = Resolver::new(root.path(), Path::new("."));
        let (file, _) = with_dcx(|dcx| r.classify_cli(dcx, Path::new("/abs/bar.eti")));
        let Some(File::Interface(id)) = file else { panic!() };
        assert_eq!(id.as_str(), "/abs/bar.eti", "absolute paths ignore --sourcepath");
    }

    #[test]
    fn unusable_cli_path_reports_and_skips() {
        let mut r = Resolver::new(Path::new("."), Path::new("."));
        let (file, diags) = with_dcx(|dcx| r.classify_cli(dcx, Path::new("foo.txt")));
        assert!(file.is_none());
        assert_eq!(error_count(&diags), 1);
        assert!(diags[0].message.contains("unknown file type"));
    }

    #[test]
    fn use_resolves_next_to_the_using_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("io.eti"), "print(s: int[])\n").unwrap();
        let from = SourceId::new(dir.path().join("main.eta").to_str().unwrap());

        let mut r = Resolver::new(Path::new("."), Path::new("."));
        let (iid, diags) = with_dcx(|dcx| r.resolve_use(dcx, from, "io", Span::DUMMY));
        assert_eq!(
            iid.expect("should resolve").expect("should resolve").as_str(),
            dir.path().join("io.eti").to_str().unwrap()
        );
        assert_eq!(error_count(&diags), 0);
    }

    #[test]
    fn use_falls_back_to_libpath() {
        let src = tempfile::tempdir().unwrap(); // no io.eti here
        let lib = tempfile::tempdir().unwrap();
        std::fs::write(lib.path().join("io.eti"), "print(s: int[])\n").unwrap();
        let from = SourceId::new(src.path().join("main.eta").to_str().unwrap());

        let mut r = Resolver::new(Path::new("."), lib.path());
        let (iid, diags) = with_dcx(|dcx| r.resolve_use(dcx, from, "io", Span::DUMMY));
        assert_eq!(
            iid.expect("should resolve via libpath").expect("should resolve via libpath").as_str(),
            lib.path().join("io.eti").to_str().unwrap()
        );
        assert_eq!(error_count(&diags), 0);
    }

    #[test]
    fn missing_use_reports_every_searched_location() {
        let src = tempfile::tempdir().unwrap();
        let lib = tempfile::tempdir().unwrap();
        let from = SourceId::new(src.path().join("main.eta").to_str().unwrap());

        let mut r = Resolver::new(Path::new("."), lib.path());
        let (iid, diags) = with_dcx(|dcx| r.resolve_use(dcx, from, "io", Span::DUMMY));
        assert!(iid.is_err());
        assert_eq!(error_count(&diags), 1);
        assert!(diags[0].message.contains("cannot find interface `io`"));
        let note = diags[0].note.as_deref().expect("searched list");
        assert!(note.contains(src.path().join("io.eti").to_str().unwrap()));
        assert!(note.contains(lib.path().join("io.eti").to_str().unwrap()));
    }

    #[test]
    fn same_interface_is_resolved_once_across_entry_points() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("io.eti"), "print(s: int[])\n").unwrap();
        let cli_path = dir.path().join("io.eti");
        let from = SourceId::new(dir.path().join("main.eta").to_str().unwrap());

        let mut r = Resolver::new(Path::new("."), Path::new("."));
        let ((cli, via_use, again), diags) = with_dcx(|dcx| {
            (
                r.classify_cli(dcx, &cli_path),
                r.resolve_use(dcx, from, "io", Span::DUMMY),
                r.resolve_use(dcx, from, "io", Span::DUMMY),
            )
        });
        assert!(matches!(cli, Some(File::Interface(_))), "first mention wins");
        assert!(via_use.is_ok() && via_use.unwrap().is_none(), "use of a CLI-queued interface is deduped");
        assert!(again.is_ok() && again.unwrap().is_none(), "repeated use is deduped");
        assert_eq!(error_count(&diags), 0, "dedup is silent, not an error");
    }
}
