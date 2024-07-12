use crate::data::FlatStateData;
use crate::state::*;
use borsh::{BorshDeserialize, BorshSerialize};
use fastnear_primitives::near_indexer_primitives::CryptoHash;
use fastnear_primitives::near_primitives::views::BlockHeaderInnerLiteView;
use std::fs::File;
use std::io::{BufWriter, Write};

const FLAT_STATE_VERSION: u8 = 1;

impl FlatState {
    fn inner_save(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        let mut writer = BufWriter::new(&mut file);

        FLAT_STATE_VERSION.serialize(&mut writer)?;

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

        let version: u8 = u8::deserialize_reader(&mut reader)?;

        if version != FLAT_STATE_VERSION {
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
