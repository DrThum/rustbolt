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

pub fn interpolate_height_map(height_map: &Vec<f32>) -> Vec<f32> {
    assert!(height_map.len() == 145);
    let mut interpolated_nested: Vec<Vec<f32>> = Vec::new();

    for (idx, &height) in height_map.iter().enumerate() {
        if (idx + 17 - 8) % 17 == 0 {
            // End of outer verticex row (8, 25, 42, ...): nothing to interpolate
            interpolated_nested.push(vec![height]);
        } else if idx % 17 < 8 {
            // Outer vertices row (0-7, 17-24, ...): interpolate the mean between the current point and the next one
            let mean = (height + height_map[idx + 1]) / 2.0;
            interpolated_nested.push(vec![height, mean]);
        } else {
            // Inner vertices (9-16, 26-33, ...)
            if idx % 17 == 9 {
                // First inner vertex of the row, interpolate before and after
                interpolated_nested.push(vec![
                    (height_map[idx - 9] + height_map[idx + 8]) / 2.0,
                    height,
                    (height_map[idx - 8] + height_map[idx + 9]) / 2.0,
                ]);
            } else {
                // Other vertices, only interpolate after
                interpolated_nested.push(vec![
                    height,
                    (height_map[idx - 8] + height_map[idx + 9]) / 2.0,
                ]);
            }
        }
    }

    let flattened: Vec<f32> = interpolated_nested.into_iter().flatten().collect();
    assert!(flattened.len() == 17 * 17);
    flattened
}
