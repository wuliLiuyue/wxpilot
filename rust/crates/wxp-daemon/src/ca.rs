// SPDX-License-Identifier: AGPL-3.0-or-later
// wxpilot - 面向 AI Agent 的微信小程序自动化 CLI
// Copyright (C) 2026  wuliLiuyue
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{
    io,
    path::{Path, PathBuf},
    process::Command,
};

use rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, IsCa, KeyUsagePurpose,
};

pub fn ensure_ca_cert(ca_dir: &Path) -> io::Result<PathBuf> {
    std::fs::create_dir_all(ca_dir)?;
    let cert_file = ca_dir.join("cert.pem");
    let key_file = ca_dir.join("key.pem");

    let should_regenerate = match (
        std::fs::read_to_string(&cert_file),
        std::fs::read_to_string(&key_file),
    ) {
        (Ok(cert_pem), Ok(key_pem)) => {
            let params = CertificateParams::from_ca_cert_pem(&cert_pem).map_err(io::Error::other);
            !matches!(params, Ok(params) if matches!(params.is_ca, IsCa::Ca(_)))
                || rcgen::KeyPair::from_pem(&key_pem).is_err()
        }
        _ => true,
    };

    if should_regenerate {
        let mut params =
            CertificateParams::new(vec!["wxpilot.local".to_string()]).map_err(io::Error::other)?;
        let mut distinguished_name = DistinguishedName::new();
        distinguished_name.push(DnType::CommonName, "wxpilot CA");
        params.distinguished_name = distinguished_name;
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![
            KeyUsagePurpose::KeyCertSign,
            KeyUsagePurpose::DigitalSignature,
            KeyUsagePurpose::CrlSign,
        ];
        let key_pair = rcgen::KeyPair::generate().map_err(io::Error::other)?;
        let cert = params.self_signed(&key_pair).map_err(io::Error::other)?;
        std::fs::write(&cert_file, cert.pem())?;
        std::fs::write(&key_file, key_pair.serialize_pem())?;
    }

    Ok(cert_file)
}

pub fn install_ca_cert(cert_file: &Path) -> io::Result<()> {
    // Integration tests use temp directories and should not mutate the host keychain.
    if cert_file.starts_with(std::env::temp_dir()) {
        return Ok(());
    }

    let keychain = format!(
        "{}/Library/Keychains/login.keychain-db",
        std::env::var("HOME").unwrap_or_default()
    );
    let status = Command::new("security")
        .args([
            "add-trusted-cert",
            "-d",
            "-r",
            "trustRoot",
            "-k",
            &keychain,
            cert_file.to_str().unwrap_or_default(),
        ])
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other("security add-trusted-cert failed"))
    }
}
