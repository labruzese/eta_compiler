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
//! Loading a file stores its text into the compilation's [`EtaCache`] — a
//! [`FileId`] only exists once its text is in the cache — so a `Resolver` is
//! bound to one cache for its whole life.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use etac_cache::{EtaCache, FileId, InterfaceId, SourceId, Span};
use etac_errors::{Diag, DiagCtxt, etac_error};

/// A classified command-line input.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum File<'ec> {
    Program(SourceId<'ec>),
    Interface(InterfaceId<'ec>),
}

pub struct Resolver<'ec> {
    source_path: PathBuf,
    lib_path: PathBuf,
    /// Every [`FileId`] handed out so far.
    seen: HashSet<FileId<'ec>>,
}

impl<'ec> Resolver<'ec> {
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
    /// `Ok(None)` means the path names a file that is already queued.
    /// `Err` carries an unemitted diagnostic: the path was unusable (non-UTF8
    /// name, unknown extension, or an I/O error).
    pub fn classify_cli<'dcx>(
        &mut self,
        dcx: &'dcx DiagCtxt<'ec>,
        path: &Path,
    ) -> Result<Option<File<'ec>>, Diag<'dcx>> {
        let path = resolve_against(&self.source_path, path);
        let Some(path_str) = path.to_str() else {
            return Err(dcx.err_no_span(format!("non-UTF8 file name {}", path.to_string_lossy())));
        };

        let is_interface = match path.extension().and_then(|x| x.to_str()) {
            Some("eta") => false,
            Some("eti") => true,
            ext => {
                return Err(dcx.err_no_span(format!(
                    "unknown file type `{}` for {path_str}",
                    ext.unwrap_or("")
                )));
            }
        };

        let id = self.load(dcx, path_str)?;
        match self.seen.insert(id) {
            true if is_interface => Ok(Some(File::Interface(id))),
            true => Ok(Some(File::Program(id))),
            false => Ok(None),
        }
    }

    /// Resolve one `use name` appearing in `from`. The search order is the directory
    /// of `from`, then `--libpath`.
    ///
    /// Takes the name and blame span rather than an AST node, so the resolver
    /// stays independent of `etac_ast` and trivially testable.
    ///
    /// `Ok(None)` means the interface is already queued. `Err` carries an
    /// unemitted diagnostic: no candidate exists on the search path (blamed at
    /// `at`, naming every location searched).
    pub fn resolve_use<'dcx>(
        &mut self,
        dcx: &'dcx DiagCtxt<'ec>,
        from: SourceId<'ec>,
        name: &str,
        at: Span,
    ) -> Result<Option<InterfaceId<'ec>>, Diag<'dcx>> {
        let file_name = format!("{name}.eti");
        let from_path = dcx.cache().source_name(from).to_string();
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
                let iid = self.load(dcx, candidate_str)?;
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
        })
    }

    /// Read `path_str` from disk and store it in the cache, reusing the
    /// existing [`FileId`] if this path has already been loaded.
    fn load<'dcx>(&self, dcx: &'dcx DiagCtxt<'ec>, path_str: &str) -> Result<FileId<'ec>, Diag<'dcx>> {
        let cache: &'ec EtaCache = dcx.cache();
        if let Some(id) = cache.source_id(path_str) {
            return Ok(id);
        }
        match std::fs::read_to_string(path_str) {
            Ok(contents) => Ok(cache.store_source(path_str.to_string(), contents).0),
            Err(ioe) => Err(Diag::io(dcx, &ioe)),
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
    use etac_cache::EtaCache;
    use etac_errors::{BufferEmitter, DiagCtxt, Level, RecordedDiag};

    /// Run `f` with a fresh cache and a context whose diagnostics are captured
    /// instead of printed, returning whatever it produced plus the recorded
    /// diagnostics. Each call gets an isolated [`EtaCache`], so tests can't
    /// see each other's files.
    fn with_dcx<T>(f: impl for<'ec> FnOnce(&DiagCtxt<'ec>) -> T) -> (T, Vec<RecordedDiag>) {
        let buf = BufferEmitter::new();
        let cache = EtaCache::new();
        let out = {
            let dcx = DiagCtxt::with_emitter(&cache, Box::new(buf.clone()));
            f(&dcx)
        };
        (out, buf.take())
    }

    fn error_count(diags: &[RecordedDiag]) -> usize {
        diags.iter().filter(|d| d.level == Level::Error).count()
    }

    /// `classify_cli`, with the driver's emit-on-error behavior folded in.
    fn classify<'ec>(r: &mut Resolver<'ec>, dcx: &DiagCtxt<'ec>, path: &Path) -> Option<File<'ec>> {
        match r.classify_cli(dcx, path) {
            Ok(file) => file,
            Err(diag) => {
                diag.emit();
                None
            }
        }
    }

    #[test]
    fn sourcepath_prefixes_relative_cli_paths() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("sub")).unwrap();
        std::fs::write(root.path().join("sub/foo.eta"), "main() {}\n").unwrap();

        let (name, diags) = with_dcx(|dcx| {
            let mut r = Resolver::new(root.path(), Path::new("."));
            let file = classify(&mut r, dcx, Path::new("sub/foo.eta"));
            let Some(File::Program(id)) = file else {
                panic!("expected a program, got {file:?}")
            };
            dcx.cache().source_name(id).to_string()
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

        let (name, _) = with_dcx(|dcx| {
            let mut r = Resolver::new(root.path(), Path::new("."));
            let file = classify(&mut r, dcx, &abs_path);
            let Some(File::Interface(id)) = file else { panic!() };
            dcx.cache().source_name(id).to_string()
        });
        assert_eq!(name, abs_path.to_str().unwrap(), "absolute paths ignore --sourcepath");
    }

    #[test]
    fn unusable_cli_path_reports_and_skips() {
        let (file, diags) = with_dcx(|dcx| {
            let mut r = Resolver::new(Path::new("."), Path::new("."));
            classify(&mut r, dcx, Path::new("foo.txt")).is_some()
        });
        assert!(!file);
        assert_eq!(error_count(&diags), 1);
        assert!(diags[0].message.contains("unknown file type"));
    }

    #[test]
    fn missing_cli_file_reports_io_error() {
        let root = tempfile::tempdir().unwrap();
        let (file, diags) = with_dcx(|dcx| {
            let mut r = Resolver::new(root.path(), Path::new("."));
            classify(&mut r, dcx, Path::new("missing.eta")).is_some()
        });
        assert!(!file);
        assert_eq!(error_count(&diags), 1);
    }

    #[test]
    fn use_resolves_next_to_the_using_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("io.eti"), "print(s: int[])\n").unwrap();
        std::fs::write(dir.path().join("main.eta"), "main() {}\n").unwrap();

        let (name, diags) = with_dcx(|dcx| {
            let mut r = Resolver::new(Path::new("."), Path::new("."));
            let from = match classify(&mut r, dcx, &dir.path().join("main.eta")) {
                Some(File::Program(id)) => id,
                _ => unreachable!(),
            };
            let iid = r
                .resolve_use(dcx, from, "io", Span::DUMMY)
                .unwrap_or_else(|d| panic!("should resolve: {}", {
                    let m = d.message.clone();
                    d.cancel();
                    m
                }))
                .expect("should resolve");
            dcx.cache().source_name(iid).to_string()
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

        let (name, diags) = with_dcx(|dcx| {
            let mut r = Resolver::new(Path::new("."), lib.path());
            let from = match classify(&mut r, dcx, &src.path().join("main.eta")) {
                Some(File::Program(id)) => id,
                _ => unreachable!(),
            };
            let iid = r
                .resolve_use(dcx, from, "io", Span::DUMMY)
                .unwrap_or_else(|d| panic!("should resolve via libpath: {}", {
                    let m = d.message.clone();
                    d.cancel();
                    m
                }))
                .expect("should resolve via libpath");
            dcx.cache().source_name(iid).to_string()
        });
        assert_eq!(name, lib.path().join("io.eti").to_str().unwrap());
        assert_eq!(error_count(&diags), 0);
    }

    #[test]
    fn missing_use_reports_every_searched_location() {
        let src = tempfile::tempdir().unwrap();
        let lib = tempfile::tempdir().unwrap();
        std::fs::write(src.path().join("main.eta"), "main() {}\n").unwrap();

        let (result_is_err, diags) = with_dcx(|dcx| {
            let mut r = Resolver::new(Path::new("."), lib.path());
            let from = match classify(&mut r, dcx, &src.path().join("main.eta")) {
                Some(File::Program(id)) => id,
                _ => unreachable!(),
            };
            match r.resolve_use(dcx, from, "io", Span::DUMMY) {
                Ok(_) => false,
                Err(diag) => {
                    diag.emit();
                    true
                }
            }
        });
        assert!(result_is_err);
        assert_eq!(error_count(&diags), 1);
        let note_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.message.contains("cannot find interface `io`"))
            .collect();
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

        let ((cli_is_interface, via_use, again), diags) = with_dcx(|dcx| {
            let mut r = Resolver::new(Path::new("."), Path::new("."));
            let cli = classify(&mut r, dcx, &cli_path);
            let from = match classify(&mut r, dcx, &dir.path().join("main.eta")) {
                Some(File::Program(id)) => id,
                _ => unreachable!(),
            };
            (
                matches!(cli, Some(File::Interface(_))),
                r.resolve_use(dcx, from, "io", Span::DUMMY).map(|o| o.is_none()).map_err(Diag::cancel),
                r.resolve_use(dcx, from, "io", Span::DUMMY).map(|o| o.is_none()).map_err(Diag::cancel),
            )
        });
        assert!(cli_is_interface, "first mention wins");
        assert!(via_use.is_ok_and(|deduped| deduped), "use of a CLI-queued interface is deduped");
        assert!(again.is_ok_and(|deduped| deduped), "repeated use is deduped");
        assert_eq!(error_count(&diags), 0, "dedup is silent, not an error");
    }
}
