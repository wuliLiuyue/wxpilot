use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Clone)]
pub struct Selector(Rc<dyn Fn(&[PathBuf]) -> Result<usize, DetectProjectError>>);

impl Selector {
    pub fn new<F>(selector: F) -> Self
    where
        F: Fn(&[PathBuf]) -> Result<usize, DetectProjectError> + 'static,
    {
        Self(Rc::new(selector))
    }

    pub fn fixed(index: usize) -> Self {
        Self::new(move |_| Ok(index))
    }

    pub fn choose(&self, candidates: &[PathBuf]) -> Result<usize, DetectProjectError> {
        (self.0)(candidates)
    }
}

#[derive(Debug)]
pub enum DetectProjectError {
    NotFound,
    MultipleCandidates { candidates: Vec<PathBuf> },
    ScanFailed { path: PathBuf, source: io::Error },
    SelectionFailed { message: String },
}

impl fmt::Display for DetectProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(
                f,
                "未找到项目路径，请传入 <projectPath> 或在包含 dist/project.config.json 的目录下运行"
            ),
            Self::MultipleCandidates { candidates } => write!(
                f,
                "未找到唯一项目路径，请显式传入 <projectPath>，候选项: {}",
                candidates
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::ScanFailed { path, source } => {
                write!(f, "扫描项目路径失败: {} ({})", path.display(), source)
            }
            Self::SelectionFailed { message } => write!(f, "选择项目路径失败: {}", message),
        }
    }
}

impl std::error::Error for DetectProjectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::ScanFailed { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl DetectProjectError {
    fn scan_failed(path: &Path, source: io::Error) -> Self {
        Self::ScanFailed {
            path: path.to_path_buf(),
            source,
        }
    }
}

pub fn detect_or_select_project(
    root: &Path,
    is_tty: bool,
    selector: Selector,
) -> Result<PathBuf, DetectProjectError> {
    let candidates = find_dist_candidates(root, 2)?;

    match candidates.as_slice() {
        [] => Err(DetectProjectError::NotFound),
        [only] => Ok(only.clone()),
        many if is_tty => {
            let index = selector.choose(many)?;
            many.get(index)
                .cloned()
                .ok_or_else(|| DetectProjectError::SelectionFailed {
                    message: format!("selector returned invalid index: {}", index),
                })
        }
        many => Err(DetectProjectError::MultipleCandidates {
            candidates: many.to_vec(),
        }),
    }
}

fn find_dist_candidates(root: &Path, depth: usize) -> Result<Vec<PathBuf>, DetectProjectError> {
    let mut candidates = Vec::new();
    collect_dist_candidates(root, 0, depth, &mut candidates)?;
    candidates.sort();
    Ok(candidates)
}

fn collect_dist_candidates(
    root: &Path,
    level: usize,
    max_level: usize,
    candidates: &mut Vec<PathBuf>,
) -> Result<(), DetectProjectError> {
    let entries =
        fs::read_dir(root).map_err(|source| DetectProjectError::scan_failed(root, source))?;

    for entry in entries {
        let entry = entry.map_err(|source| DetectProjectError::scan_failed(root, source))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|source| DetectProjectError::scan_failed(&path, source))?;
        if !file_type.is_dir() {
            continue;
        }

        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        if name == "dist" {
            let config_path = path.join("project.config.json");
            match config_path.try_exists() {
                Ok(true) => {
                    candidates.push(path);
                    continue;
                }
                Ok(false) => continue,
                Err(source) => return Err(DetectProjectError::scan_failed(&config_path, source)),
            }
        }

        if level < max_level && !name.starts_with('.') && name != "node_modules" {
            collect_dist_candidates(&path, level + 1, max_level, candidates)?;
        }
    }

    Ok(())
}
