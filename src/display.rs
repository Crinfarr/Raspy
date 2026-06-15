pub mod inland_fpc_a002;

pub trait PeripheralDisplay<C> {
    const DIMENSIONS: [u16; 2];
    const BITS_PER_PIXEL: u8;
    async fn show_img(&self, px: &[u8]) -> std::io::Result<()>;
    async fn draw_px(&self, x: u8, y: u8, color: C) -> std::io::Result<()>;
}
pub trait PartialRenderCapableDisplay<T, C>: PeripheralDisplay<C> {
    ///[XMin, XMax, YMin, YMax]
    const ORIGINAL_WINDOW: [T; 4];
    ///window: &[XMin, XMax, YMin, YMax]
    async fn partial_upd(&self, window: &[T; 4], px: &[u8]) -> std::io::Result<()>;
}
