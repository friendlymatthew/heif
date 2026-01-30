use crate::{cabac::ArithmeticDecoderEngine, hevc::RbspReader};
use anyhow::Result;

#[derive(Debug)]
pub struct CabacDecoder<'a, 'b> {
    engine: ArithmeticDecoderEngine<'a, 'b>,
}

impl<'a, 'b> CabacDecoder<'a, 'b> {
    pub fn try_new(
        reader: &'a mut RbspReader<'b>,

        init_qp_minus26: i32,
        slice_qp_delta: i32,
    ) -> Result<Self> {
        let slice_qp = 26 + init_qp_minus26 + slice_qp_delta;

        let mut engine = ArithmeticDecoderEngine::try_new(reader)?;
        engine.init_all_contexts(slice_qp);

        Ok(Self { engine })
    }
}
