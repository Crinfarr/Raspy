pub mod inland_fpc_a002;

pub trait PeripheralDisplay {
    const DIMENSIONS: [u16; 2];
    const BITS_PER_PIXEL: u8;
    async fn show_img(&self, px: &[u8]) -> std::io::Result<()>;
}
pub trait PartialRenderCapableDisplay<T>: PeripheralDisplay {
    ///[XMin, XMax, YMin, YMax]
    const ORIGINAL_WINDOW: [T; 4];
    ///window: &[XMin, XMax, YMin, YMax]
    async fn partial_upd(&self, window: &[T; 4], px: &[u8]) -> std::io::Result<()>;
}
