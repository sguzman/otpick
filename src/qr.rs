use std::io;
use std::path::Path;

use image::{GrayImage, ImageReader};
use zeroize::Zeroizing;

pub fn decode_file(path: &Path) -> Result<Vec<Zeroizing<String>>, QrError> {
    let image = ImageReader::open(path)?
        .with_guessed_format()?
        .decode()?
        .to_luma8();

    decode_image(image)
}

fn decode_image(image: GrayImage) -> Result<Vec<Zeroizing<String>>, QrError> {
    let mut prepared = rqrr::PreparedImage::prepare(image);
    let grids = prepared.detect_grids();

    if grids.is_empty() {
        return Err(QrError::NotFound);
    }

    let mut payloads = Vec::with_capacity(grids.len());
    for grid in grids {
        let (_, payload) = grid
            .decode()
            .map_err(|error| QrError::Decode(error.to_string()))?;
        payloads.push(Zeroizing::new(payload));
    }

    Ok(payloads)
}

#[derive(Debug, thiserror::Error)]
pub enum QrError {
    #[error("could not read QR image: {0}")]
    Io(#[from] io::Error),
    #[error("could not decode image: {0}")]
    Image(#[from] image::ImageError),
    #[error("no QR code found in image")]
    NotFound,
    #[error("QR code could not be decoded: {0}")]
    Decode(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Luma;
    use qrcode::QrCode;

    #[test]
    fn decodes_rendered_qr_payload() {
        let payload = "otpauth://totp/GitHub:alice?secret=GEZDGNBVGY3TQOJQ&issuer=GitHub";
        let code = QrCode::new(payload.as_bytes()).unwrap();
        let image = code.render::<Luma<u8>>().min_dimensions(512, 512).build();

        let decoded = decode_image(image).unwrap();

        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].as_str(), payload);
    }

    #[test]
    fn blank_image_has_no_qr() {
        let image = GrayImage::from_pixel(256, 256, Luma([255]));

        assert!(matches!(decode_image(image), Err(QrError::NotFound)));
    }
}
