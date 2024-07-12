use crate::data::FlatStateData;
use crate::state::*;
use borsh::{BorshDeserialize, BorshSerialize};
use fastnear_primitives::near_indexer_primitives::CryptoHash;
use fastnear_primitives::near_primitives::account::{AccessKey, Account};
use fastnear_primitives::near_primitives::types::AccountId;
use fastnear_primitives::near_primitives::views::BlockHeaderInnerLiteView;
use near_crypto::PublicKey;
use psutil::process::Process;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Seek, Write};

const FLAT_STATE_VERSION: u8 = 0;

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
        let current_pid = psutil::process::Process::current().unwrap().pid();
        let process = Process::new(current_pid).unwrap();

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
        let mut prev_offset = reader.stream_position()?;
        let mut prev_ram = process.memory_info().unwrap().rss();
        let access_keys =
            HashMap::<AccountId, Vec<(PublicKey, AccessKey)>>::deserialize_reader(&mut reader)?;
        let offset = reader.stream_position()?;
        let ram = process.memory_info().unwrap().rss();
        println!(
            "Data Size (Access Keys): {} Ram used {}",
            offset - prev_offset,
            ram.saturating_sub(prev_ram)
        );
        prev_offset = offset;
        prev_ram = ram;
        let accounts = HashMap::<AccountId, Account>::deserialize_reader(&mut reader)?;
        let offset = reader.stream_position()?;
        let ram = process.memory_info().unwrap().rss();
        println!(
            "Data Size (Accounts): {} Ram used {}",
            offset - prev_offset,
            ram.saturating_sub(prev_ram)
        );
        prev_offset = offset;
        prev_ram = ram;
        let data =
            HashMap::<AccountId, HashMap<Vec<u8>, Vec<u8>>>::deserialize_reader(&mut reader)?;
        let offset = reader.stream_position()?;
        let ram = process.memory_info().unwrap().rss();
        println!(
            "Data Size (Data): {} Ram used {}",
            offset - prev_offset,
            ram.saturating_sub(prev_ram)
        );
        prev_offset = offset;
        prev_ram = ram;
        let contracts_code = HashMap::<AccountId, Vec<u8>>::deserialize_reader(&mut reader)?;
        let offset = reader.stream_position()?;
        let ram = process.memory_info().unwrap().rss();
        println!(
            "Data Size (Contracts Code): {} Ram used {}",
            offset - prev_offset,
            ram.saturating_sub(prev_ram)
        );

        Ok(Self {
            config,
            block_header,
            block_hash,
            data: FlatStateData {
                access_keys,
                accounts,
                data,
                contracts_code,
            },
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
