#[derive(Debug)]
pub struct ICCProfile {}

#[derive(Debug)]
pub struct ICCProfileHeader {}

#[derive(Debug)]
pub enum ProfileClass {
    InputDevice,
    DisplayDevice,
    OutputDevice,
    DeviceLink,
    ColorSpace,
    Abstract,
    NamedColor,
}

#[derive(Debug)]
pub enum ColorSpace {
    CIEXYZ,
    CIELAB,
    CIELUV,
    YCbCr,
    CIEYxy,
    RGB,
    Gray,
    HSV,
    HLS,
    CMYK,
    CMY,
    Color(u8),
}

#[derive(Debug)]
pub enum PrimaryPlatform {
    Apple,
    Microsoft,
    Silicon,
    Sun,
    General,
}
