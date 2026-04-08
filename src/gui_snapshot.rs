use std::{fs::File, io::BufWriter, path::Path};

pub fn write_display_png<P: AsRef<Path>>(
    path: P,
    width: usize,
    height: usize,
    depth: usize,
    bytes: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width as u32, height as u32);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    let raster = match depth {
        1 => expand_monochrome(width, height, bytes),
        8 => bytes.to_vec(),
        _ => return Err(format!("unsupported display depth {depth}").into()),
    };
    writer.write_image_data(&raster)?;
    Ok(())
}

fn expand_monochrome(width: usize, height: usize, bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(width * height);
    for pixel_index in 0..(width * height) {
        let byte = bytes[pixel_index / 8];
        let bit = 1u8 << (7 - (pixel_index % 8));
        let is_set = (byte & bit) != 0;
        out.push(if is_set { 0 } else { 255 });
    }
    out
}
