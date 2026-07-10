//! Path resolution
//!
//! Everything that turns a user-facing name into a [`FileId`] lives here:
//!
//! * command-line paths -> classified [`File`]s, with relative paths resolved
//!   against `--sourcepath`;
//! * `use F` declarations -> `F.eti`, searched next to the
//!   using source file first and then under `--libpath`;
//! * interface deduplication
//!
//! Failures are reported here.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use etac_errors::{Diag, DiagCtxt, etac_error, ErrorGuaranteed};
use etac_span::{FileId, InterfaceId, SourceCache, SourceId, Span};

/// A classified command-line input.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum File {
    Program(SourceId),
    Interface(InterfaceId),
}

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

    /// Classify and load a file named on the command line, resolving relative
    /// paths against `--sourcepath`.
    ///
    /// Unlike path classification alone, this also reads the file's contents and
    /// stores them into `dcx`'s [`SourceCache`] — a [`FileId`] only exists once its
    /// text is in the cache, so loading can no longer be deferred to a later phase.
    ///
    /// `None` means skip: the path was unusable (non-UTF8 name, unknown
    /// extension, or an I/O error — all reported) or names a file that is
    /// already queued (silent).
    pub fn classify_cli<C: SourceCache>(&mut self, dcx: &DiagCtxt<C>, path: &Path) -> Option<File> {
        let path = resolve_against(&self.source_path, path);
        let Some(path_str) = path.to_str() else {
            dcx.err_no_span(format!("non-UTF8 file name {}", path.to_string_lossy()))
                .emit();
            return None;
        };

        let is_interface = match path.extension().and_then(|x| x.to_str()) {
            Some("eta") => false,
            Some("eti") => true,
            ext => {
                dcx.err_no_span(format!(
                    "unknown file type `{}` for {path_str}",
                    ext.unwrap_or("")
                ))
                .emit();
                return None;
            }
        };

        let id = self.load(dcx, path_str)?;
        self.seen.insert(id).then_some(if is_interface {
            File::Interface(id)
        } else {
            File::Program(id)
        })
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
    pub fn resolve_use<C: SourceCache>(
        &mut self,
        dcx: &DiagCtxt<C>,
        from: SourceId,
        name: &str,
        at: Span,
    ) -> Result<Option<InterfaceId>, ErrorGuaranteed> {
        let file_name = format!("{name}.eti");
        let from_path = dcx.sources().load_name(from).to_string();
        let from_dir = Path::new(&from_path)
            .parent()
            .unwrap_or_else(|| Path::new(""));

        let mut candidates = vec![from_dir.join(&file_name)];
        let in_lib = self.lib_path.join(&file_name);
        if in_lib != candidates[0] {
            candidates.push(in_lib);
        }

        for candidate in &candidates {
            if candidate.is_file() {
                let Some(candidate_str) = candidate.to_str() else {
                    continue;
                };
                let Some(iid) = self.load(dcx, candidate_str) else {
                    // load() already emitted a diagnostic for a real I/O error; a
                    // non-UTF8 path can't happen here since `candidate_str` succeeded.
                    return Err(unsafe { ErrorGuaranteed::claim_already_emitted() });
                };
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

    /// Read `path_str` from disk and store it in `dcx`'s cache, reusing the existing
    /// [`FileId`] if this path has already been loaded. `None` means an I/O error was
    /// hit and already reported.
    fn load<C: SourceCache>(&self, dcx: &DiagCtxt<C>, path_str: &str) -> Option<FileId> {
        if let Some(id) = dcx.sources().contains(path_str) {
            return Some(id);
        }
        match std::fs::read_to_string(path_str) {
            Ok(contents) => Some(dcx.sources().store(path_str.to_string(), contents).0),
            Err(ioe) => {
                Diag::io(dcx, &ioe).emit();
                None
            }
        }
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
    use etac_span::SCache;

    /// Run `f` with a context whose diagnostics are captured instead of
    /// printed, returning whatever it produced plus the recorded diagnostics.
    /// Each call gets a fresh, isolated [`GlobalCache`] rather than the
    /// process-wide singleton, so tests can't see each other's files.
    fn with_dcx<T>(f: impl FnOnce(&DiagCtxt<SCache>) -> T) -> (T, Vec<RecordedDiag>) {
        let buf = BufferEmitter::new();
        let out = {
            let dcx = DiagCtxt::with_emitter(SCache::default(), Box::new(buf.clone()));
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
        std::fs::create_dir(root.path().join("sub")).unwrap();
        std::fs::write(root.path().join("sub/foo.eta"), "main() {}\n").unwrap();

        let mut r = Resolver::new(root.path(), Path::new("."));
        let (name, diags) = with_dcx(|dcx| {
            let file = r.classify_cli(dcx, Path::new("sub/foo.eta"));
            let Some(File::Program(id)) = file else {
                panic!("expected a program, got {file:?}")
            };
            dcx.sources().load_name(id).to_string()
        });
        assert_eq!(name, root.path().join("sub/foo.eta").to_str().unwrap());
        assert_eq!(error_count(&diags), 0);
    }

    #[test]
    fn default_sourcepath_leaves_relative_paths_verbatim() {
        assert_eq!(
            resolve_against(Path::new("."), Path::new("sub/foo.eta")),
            Path::new("sub/foo.eta"),
            "default `.` must not rewrite the path"
        );
    }

    #[test]
    fn absolute_cli_paths_ignore_sourcepath() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("bar.eti"), "\n").unwrap();
        let abs_path = root.path().join("bar.eti");

        let mut r = Resolver::new(root.path(), Path::new("."));
        let (name, _) = with_dcx(|dcx| {
            let file = r.classify_cli(dcx, &abs_path);
            let Some(File::Interface(id)) = file else { panic!() };
            dcx.sources().load_name(id).to_string()
        });
        assert_eq!(name, abs_path.to_str().unwrap(), "absolute paths ignore --sourcepath");
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
    fn missing_cli_file_reports_io_error() {
        let root = tempfile::tempdir().unwrap();
        let mut r = Resolver::new(root.path(), Path::new("."));
        let (file, diags) = with_dcx(|dcx| r.classify_cli(dcx, Path::new("missing.eta")));
        assert!(file.is_none());
        assert_eq!(error_count(&diags), 1);
    }

    #[test]
    fn use_resolves_next_to_the_using_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("io.eti"), "print(s: int[])\n").unwrap();
        std::fs::write(dir.path().join("main.eta"), "main() {}\n").unwrap();

        let mut r = Resolver::new(Path::new("."), Path::new("."));
        let (name, diags) = with_dcx(|dcx| {
            let from = match r.classify_cli(dcx, &dir.path().join("main.eta")) {
                Some(File::Program(id)) => id,
                _ => unreachable!(),
            };
            let iid = r
                .resolve_use(dcx, from, "io", Span::DUMMY)
                .expect("should resolve")
                .expect("should resolve");
            dcx.sources().load_name(iid).to_string()
        });
        assert_eq!(name, dir.path().join("io.eti").to_str().unwrap());
        assert_eq!(error_count(&diags), 0);
    }

    #[test]
    fn use_falls_back_to_libpath() {
        let src = tempfile::tempdir().unwrap(); // no io.eti here
        let lib = tempfile::tempdir().unwrap();
        std::fs::write(lib.path().join("io.eti"), "print(s: int[])\n").unwrap();
        std::fs::write(src.path().join("main.eta"), "main() {}\n").unwrap();

        let mut r = Resolver::new(Path::new("."), lib.path());
        let (name, diags) = with_dcx(|dcx| {
            let from = match r.classify_cli(dcx, &src.path().join("main.eta")) {
                Some(File::Program(id)) => id,
                _ => unreachable!(),
            };
            let iid = r
                .resolve_use(dcx, from, "io", Span::DUMMY)
                .expect("should resolve via libpath")
                .expect("should resolve via libpath");
            dcx.sources().load_name(iid).to_string()
        });
        assert_eq!(name, lib.path().join("io.eti").to_str().unwrap());
        assert_eq!(error_count(&diags), 0);
    }

    #[test]
    fn missing_use_reports_every_searched_location() {
        let src = tempfile::tempdir().unwrap();
        let lib = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("main.eta"), "main() {}\n").unwrap();

        let mut r = Resolver::new(Path::new("."), lib.path());
        let (result, diags) = with_dcx(|dcx| {
            let from = match r.classify_cli(dcx, &src.path().join("main.eta")) {
                Some(File::Program(id)) => id,
                _ => unreachable!(),
            };
            r.resolve_use(dcx, from, "io", Span::DUMMY)
        });
        assert!(result.is_err());
        assert_eq!(error_count(&diags), 1);
        let note_diags: Vec<_> = diags.iter().filter(|d| d.message.contains("cannot find interface `io`")).collect();
        assert_eq!(note_diags.len(), 1);
        let note = note_diags[0].note.as_deref().expect("searched list");
        assert!(note.contains(src.path().join("io.eti").to_str().unwrap()));
        assert!(note.contains(lib.path().join("io.eti").to_str().unwrap()));
    }

    #[test]
    fn same_interface_is_resolved_once_across_entry_points() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("io.eti"), "print(s: int[])\n").unwrap();
        std::fs::write(dir.path().join("main.eta"), "main() {}\n").unwrap();
        let cli_path = dir.path().join("io.eti");

        let mut r = Resolver::new(Path::new("."), Path::new("."));
        let ((cli, via_use, again), diags) = with_dcx(|dcx| {
            let cli = r.classify_cli(dcx, &cli_path);
            let from = match r.classify_cli(dcx, &dir.path().join("main.eta")) {
                Some(File::Program(id)) => id,
                _ => unreachable!(),
            };
            (
                cli,
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
