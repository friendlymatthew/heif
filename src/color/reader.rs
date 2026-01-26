use crate::{
    color::{
        ICCProfileHeader,
        grammar::{ColorSpace, ICCProfile, PrimaryPlatform, ProfileClass},
    },
    impl_read_for_datatype,
};
use anyhow::{Result, anyhow, bail, ensure};

#[derive(Debug)]
pub struct ICCProfileReader<'a> {
    cursor: usize,
    data: &'a [u8],
}

impl<'a> ICCProfileReader<'a> {
    pub const fn new(data: &'a [u8]) -> Self {
        Self { cursor: 0, data }
    }

    pub fn read(&mut self) -> Result<ICCProfile> {
        let _header = self.read_icc_profile_header()?;

        ensure!(self.cursor == self.data.len());

        todo!();
    }

    fn read_icc_profile_header(&mut self) -> Result<ICCProfileHeader> {
        let _profile_size = self.read_u32()?;
        let _preferred_cmm_type = self.read_u32()?;
        let _profile_version = self.read_u32()?;

        let _profile_class = match self.read_slice(4)? {
            b"scnr" => ProfileClass::InputDevice,
            b"mntr" => ProfileClass::DisplayDevice,
            b"prtr" => ProfileClass::OutputDevice,
            b"link" => ProfileClass::DeviceLink,
            b"spac" => ProfileClass::ColorSpace,
            b"abst" => ProfileClass::Abstract,
            b"nmcl" => ProfileClass::NamedColor,
            foreign => bail!(
                "encountered foreign profile class {:?}",
                str::from_utf8(foreign)
            ),
        };

        let _color_space = match self.read_slice(4)? {
            b"XYZ " => ColorSpace::CIEXYZ,
            b"Lab " => ColorSpace::CIELAB,
            b"Luv " => ColorSpace::CIELUV,
            b"YCbr" => ColorSpace::YCbCr,
            b"Yxy " => ColorSpace::CIEYxy,
            b"RGB " => ColorSpace::RGB,
            b"GRAY" => ColorSpace::Gray,
            b"HSV " => ColorSpace::HSV,
            b"HLS " => ColorSpace::HLS,
            b"CMYK" => ColorSpace::CMYK,
            b"CMY " => ColorSpace::CMY,
            b"2CLR" => ColorSpace::Color(2),
            b"3CLR" => ColorSpace::Color(3),
            b"4CLR" => ColorSpace::Color(4),
            b"5CLR" => ColorSpace::Color(5),
            b"6CLR" => ColorSpace::Color(6),
            b"7CLR" => ColorSpace::Color(7),
            b"8CLR" => ColorSpace::Color(8),
            b"9CLR" => ColorSpace::Color(9),
            b"ACLR" => ColorSpace::Color(10),
            b"BCLR" => ColorSpace::Color(11),
            b"CCLR" => ColorSpace::Color(12),
            b"DCLR" => ColorSpace::Color(13),
            b"ECLR" => ColorSpace::Color(14),
            b"FCLR" => ColorSpace::Color(15),
            foreign => bail!(
                "encountered foreign color space {:?}",
                str::from_utf8(foreign)
            ),
        };

        let _pcs_field = self.read_u32()?;

        let _created_at = self.read_date_time_number()?;

        ensure!(self.read_slice(4)? == b"acsp");

        let _primary_platform = match self.read_slice(4)? {
            b"APPL" => PrimaryPlatform::Apple,
            b"MSFT" => PrimaryPlatform::Microsoft,
            b"SGI " => PrimaryPlatform::Silicon,
            b"SUNW" => PrimaryPlatform::Sun,
            &[0, 0, 0, 0] => PrimaryPlatform::General,
            foreign => bail!("encountered foreign platform {:?}", str::from_utf8(foreign)),
        };

        let _profile_flags = self.read_u32()?;
        let _device_manufacturer = self.read_u32()?;
        let _device_model = self.read_u32()?;
        let _device_attributes = self.read_u64()?;
        let _rendering_intent = self.read_u32()?;

        // parse the rest from 7.2.16

        todo!();
    }

    fn read_slice(&mut self, len: usize) -> Result<&'a [u8]> {
        let s = self
            .data
            .get(self.cursor..self.cursor + len)
            .ok_or_else(|| anyhow!("oob"))?;

        self.cursor += len;
        Ok(s)
    }

    impl_read_for_datatype!(read_u16, u16);
    impl_read_for_datatype!(read_u32, u32);
    impl_read_for_datatype!(read_u64, u64);

    fn read_date_time_number(&mut self) -> Result<DateTime<Utc>> {
        let year = self.read_u16()?;
        let month = self.read_u16()?;
        let day = self.read_u16()?;
        let hours = self.read_u16()?;
        let minutes = self.read_u16()?;
        let seconds = self.read_u16()?;

        let naive_date = NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32)
            .ok_or_else(|| anyhow!("Invalid date: {}-{}-{}", year, month, day))?;
        let naive_time = NaiveTime::from_hms_opt(hours as u32, minutes as u32, seconds as u32)
            .ok_or_else(|| anyhow!("Invalid time: {}:{}:{}", hours, minutes, seconds))?;
        let naive_datetime = naive_date.and_time(naive_time);

        Ok(DateTime::from_naive_utc_and_offset(naive_datetime, Utc))
    }
}
