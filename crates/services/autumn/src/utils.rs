/// Convert image to sRGB using the provided ICC profile.
/// Returns the converted image, or the original if conversion fails.
pub fn apply_icc_profile(image: image::DynamicImage, icc: &[u8]) -> image::DynamicImage {
    let Ok(src_profile) = lcms2::Profile::new_icc(icc) else {
        return image;
    };
    let dst_profile = lcms2::Profile::new_srgb();
    let format = if image.color().has_alpha() {
        lcms2::PixelFormat::RGBA_8
    } else {
        lcms2::PixelFormat::RGB_8
    };
    let Ok(t) = lcms2::Transform::new(
        &src_profile,
        format,
        &dst_profile,
        format,
        lcms2::Intent::Perceptual,
    ) else {
        return image;
    };
    if image.color().has_alpha() {
        let mut rgba_image = image.into_rgba8();
        t.transform_in_place(rgba_image.as_mut());
        image::DynamicImage::ImageRgba8(rgba_image)
    } else {
        let mut rgb_image = image.into_rgb8();
        t.transform_in_place(rgb_image.as_mut());
        image::DynamicImage::ImageRgb8(rgb_image)
    }
}
