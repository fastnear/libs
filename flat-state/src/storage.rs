use crate::data::FlatStateData;
use crate::state::*;
use borsh::{BorshDeserialize, BorshSerialize};
use fastnear_primitives::near_indexer_primitives::CryptoHash;
use fastnear_primitives::near_primitives::views::BlockHeaderInnerLiteView;
use std::fs::File;
use std::io::{BufWriter, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, BorshSerialize, BorshDeserialize)]
pub enum FlatStateVersion {
    V1,
}

impl FlatState {
    fn inner_save(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        let mut writer = BufWriter::new(&mut file);
        FlatStateVersion::V1.serialize(&mut writer)?;

        self.config.serialize(&mut writer)?;
        self.block_header.serialize(&mut writer)?;
        self.block_hash.serialize(&mut writer)?;
        self.data.serialize(&mut writer)?;

        writer.flush()?;

        Ok(())
    }

    fn inner_load(path: &str) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let mut reader = std::io::BufReader::new(file);

        let version: FlatStateVersion = FlatStateVersion::deserialize_reader(&mut reader)?;

        if version != FlatStateVersion::V1 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unsupported version",
            ));
        }
        let config = FlatStateConfig::deserialize_reader(&mut reader)?;
        let block_header = BlockHeaderInnerLiteView::deserialize_reader(&mut reader)?;
        let block_hash = CryptoHash::deserialize_reader(&mut reader)?;
        let data = FlatStateData::deserialize_reader(&mut reader)?;

        Ok(Self {
            config,
            block_header,
            block_hash,
            data,
        })
    }

    pub fn save(&self, path: &str) -> FlatStateResult<()> {
        self.inner_save(path)
            .map_err(|e| FlatStateError::StorageError(e.to_string()))
    }

    pub fn load(path: &str) -> FlatStateResult<Self> {
        Self::inner_load(path).map_err(|e| FlatStateError::StorageError(e.to_string()))
    }
}
