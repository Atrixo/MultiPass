use crate::motion::{estimate, Estimate};
use crate::ncfile::{NcFile, Units};
use crate::rapid::restore_rapids;
use std::borrow::Borrow;
use std::path::Path;

pub const SAFE_Z_MIN: f64 = 0.5;
pub const SAFE_Z_MAX: f64 = 20.0;

pub fn rapid_allowed<F: Borrow<NcFile>>(files: &[F]) -> bool {
    files.iter().map(Borrow::borrow).all(|f| !f.incremental && f.is_stock_top())
}

pub fn effective_body(f: &NcFile, safe_z: Option<f64>) -> Vec<String> {
    match safe_z {
        Some(z) => match restore_rapids(f.body(), z) {
            Ok((out, _)) => out.into_iter().map(|c| c.into_owned()).collect(),
            Err(_) => f.body().to_vec(),
        },
        None => f.body().to_vec(),
    }
}

pub fn estimate_file(f: &NcFile, safe_z: Option<f64>) -> Estimate {
    let inch = f.units == Units::Inch;
    match safe_z {
        Some(_) => estimate(&effective_body(f, safe_z), inch),
        None => estimate(f.body(), inch),
    }
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let file_name =
        path.file_name().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "kein Dateiname"))?;
    let mut tmp_name = file_name.to_os_string();
    tmp_name.push(".multipass-tmp");
    let tmp = path.with_file_name(tmp_name);
    std::fs::write(&tmp, bytes)?;
    if let Err(e) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomar_schreiben_ersetzt_und_raeumt_auf() {
        let dir = std::env::temp_dir().join(format!("multipass_job_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let out = dir.join("A.cnc");
        std::fs::write(&out, "alt").unwrap();
        write_atomic(&out, b"neu").unwrap();
        assert_eq!(std::fs::read_to_string(&out).unwrap(), "neu");
        assert!(!dir.join("A.cnc.multipass-tmp").exists());
        assert!(write_atomic(&dir.join("fehlt").join("B.cnc"), b"x").is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn eilgang_nur_bei_materialoberkante_und_absolut() {
        let ok = NcFile::parse("a.nc", b"(   Z Origin Set To   : Stock Top )\nG90\nT1M6\nM30\n").unwrap();
        let other = NcFile::parse("b.nc", b"(   Z Origin Set To   : Stock Bottom )\nG90\nT1M6\nM30\n").unwrap();
        assert!(rapid_allowed(&[&ok]));
        assert!(!rapid_allowed(&[&ok, &other]));
    }
}
