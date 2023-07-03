use bevy::{
    asset::{AssetLoader, LoadContext, LoadedAsset},
    reflect::impl_type_uuid,
    utils::BoxedFuture,
};
use binrw::{binread, io::Cursor, BinReaderExt};
use shared::models::terrain_info::TerrainBlock;

#[derive(Debug)]
#[binread]
pub struct WrappedTerrainBlock(pub TerrainBlock);

impl_type_uuid!(WrappedTerrainBlock, "269b2e4a5af644e0833bd65e29f5342d");

#[derive(Default)]
pub struct TerrainBlockLoader;

impl AssetLoader for TerrainBlockLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<(), bevy::asset::Error>> {
        Box::pin(async move {
            if bytes.len() > 0 {
                let mut reader = Cursor::new(bytes);
                let terrain_block: WrappedTerrainBlock = reader.read_le().unwrap();
                load_context.set_default_asset(LoadedAsset::new(terrain_block));
                Ok(())
            } else {
                Err(anyhow::Error::msg("non-existing terrain file"))
            }
        })
    }

    fn extensions(&self) -> &[&str] {
        &["terrain"]
    }
}
