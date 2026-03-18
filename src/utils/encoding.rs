//! 模块说明：通用工具函数集合。
//!
//! 文件路径：src/utils/encoding.rs。
//! 该文件围绕 'encoding' 的职责提供实现。
//! 关键符号：decode_file、read_utf8、read_with_encoding、auto_detect。

use std::{
    fs::File,
    io::{BufReader, Read},
};

use encoding_rs::Encoding;
use encoding_rs_io::DecodeReaderBytesBuilder;
use log::{debug, info, trace, warn};

use crate::error::ImporterResult;

/// 根据编码名称解码文件内容
pub fn decode_file(file: File, encoding_name: &str) -> ImporterResult<String> {
    let encoding_name = encoding_name.to_uppercase();
    debug!("Decoding file with encoding: {}", encoding_name);

    match encoding_name.as_str() {
        "AUTO" => auto_detect(file),
        "UTF-8" | "UTF8" => read_utf8(file),
        _ => read_with_encoding(file, &encoding_name),
    }
}

/// 读取 UTF-8 文件
fn read_utf8(file: File) -> ImporterResult<String> {
    let mut reader = BufReader::new(file);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;

    match String::from_utf8(bytes.clone()) {
        Ok(content) => {
            // 移除 BOM
            let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
            Ok(content.to_string())
        }
        Err(_) => {
            warn!("UTF-8 解码失败，尝试自动检测");
            auto_detect_from_bytes(&bytes)
        }
    }
}

/// 使用指定编码读取
fn read_with_encoding(file: File, encoding_name: &str) -> ImporterResult<String> {
    let encoding = match encoding_name {
        "GBK" | "GB2312" | "GB18030" => encoding_rs::GBK,
        "BIG5" => encoding_rs::BIG5,
        "SHIFT_JIS" | "SHIFT-JIS" | "SJIS" => encoding_rs::SHIFT_JIS,
        _ => Encoding::for_label(encoding_name.as_bytes()).unwrap_or(encoding_rs::GBK),
    };

    info!("使用编码: {}", encoding.name());

    let transcoded = DecodeReaderBytesBuilder::new()
        .encoding(Some(encoding))
        .build(file);

    let mut reader = BufReader::new(transcoded);
    let mut content = String::new();
    reader.read_to_string(&mut content)?;

    Ok(content)
}

/// 自动检测编码
fn auto_detect(file: File) -> ImporterResult<String> {
    let mut reader = BufReader::new(file);
    let mut bytes = Vec::new();
    reader.read_to_end(&mut bytes)?;

    // 先尝试 UTF-8
    if let Ok(content) = String::from_utf8(bytes.clone()) {
        let content = content.strip_prefix('\u{feff}').unwrap_or(&content);
        info!("检测到编码: UTF-8");
        return Ok(content.to_string());
    }

    auto_detect_from_bytes(&bytes)
}

/// 从字节自动检测编码
fn auto_detect_from_bytes(bytes: &[u8]) -> ImporterResult<String> {
    let encodings = [
        ("GBK", encoding_rs::GBK),
        ("GB18030", encoding_rs::GB18030),
        ("BIG5", encoding_rs::BIG5),
    ];

    for (name, encoding) in encodings {
        let (decoded, _, had_errors) = encoding.decode(bytes);
        if !had_errors {
            info!("检测到编码: {}", name);
            return Ok(decoded.into_owned());
        }
        trace!("{} 解码有错误，继续尝试", name);
    }

    // 强制使用 GBK
    warn!("无法确定编码，强制使用 GBK");
    let (decoded, _, _) = encoding_rs::GBK.decode(bytes);
    Ok(decoded.into_owned())
}
