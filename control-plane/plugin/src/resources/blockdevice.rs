use crate::{
    operations::GetBlockDevices,
    resources::{
        utils::{
            print_table, CreateRows, GetHeaderRow, OutputFormat, BLOCKDEVICE_HEADERS_ALL,
            BLOCKDEVICE_HEADERS_USABLE,
        },
        NodeId,
    },
    rest_wrapper::RestClient,
};
use async_trait::async_trait;
use openapi::apis::Url;
use prettytable::Row;
use serde::Serialize;

/// Blockdevice resource.
#[derive(clap::Args, Debug)]
pub struct BlockDevice {}

/// New type to wrap BD returned from REST call with all set to true
#[derive(Clone, Debug, Serialize)]
struct BlockDeviceAll(openapi::models::BlockDevice);

/// New type to wrap BD returned from REST call with all set to false
#[derive(Clone, Debug, Serialize)]
struct BlockDeviceUsable(openapi::models::BlockDevice);

#[derive(Debug, Clone, clap::Args)]
/// BlockDevice args
pub struct BlockDeviceArgs {
    /// Id of the node
    node_id: NodeId,
    #[clap(long)]
    /// Shows all devices if invoked, otherwise shows usable disks.
    all: bool,
}

impl BlockDeviceArgs {
    /// get the node id
    pub fn node_id(&self) -> NodeId {
        self.node_id.clone()
    }
    /// get the all value
    pub fn all(&self) -> bool {
        self.all
    }
}

// CreateRows trait for BlockDeviceAll would create the row from the
// BD returned from REST call with all set to true.
impl CreateRows for BlockDeviceAll {
    fn create_rows(&self) -> Vec<Row> {
        vec![row![
            self.0.devname.clone(),
            self.0.devtype.clone(),
            self.0.size.to_string(),
            String::from(if self.0.available { "yes" } else { "no" }),
            self.0.model.clone(),
            self.0.devpath.clone(),
            self.0.devmajor.to_string(),
            self.0.devminor.to_string(),
            self.0.filesystem.fstype.clone(),
            self.0.filesystem.uuid.clone(),
            self.0.filesystem.mountpoint.clone(),
            get_partition_type(&self.0.partition),
            self.0
                .devlinks
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<String>>()
                .join(", "),
        ]]
    }
}

// CreateRows trait for BlockDeviceUsable would create row from the
// BD returned from REST call with all set to false.
impl CreateRows for BlockDeviceUsable {
    fn create_rows(&self) -> Vec<Row> {
        vec![row![
            self.0.devname.clone(),
            self.0.devtype.clone(),
            self.0.size.to_string(),
            String::from(if self.0.available { "yes" } else { "no" }),
            self.0.model.clone(),
            self.0.devpath.clone(),
            self.0.devmajor.to_string(),
            self.0.devminor.to_string(),
            self.0
                .devlinks
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<String>>()
                .join(", "),
        ]]
    }
}

fn get_partition_type(partition: &openapi::models::BlockDevicePartition) -> String {
    if !partition.scheme.is_empty() && !partition.typeid.is_empty() {
        return format!("{}:{}", partition.scheme, partition.typeid);
    }
    "".to_string()
}

// GetHeaderRow being trait for BlockDeviceAll would return the Header Row for
// BD returned from REST call with all set to true.
impl GetHeaderRow for BlockDeviceAll {
    fn get_header_row(&self) -> Row {
        (&*BLOCKDEVICE_HEADERS_ALL).clone()
    }
}

// GetHeaderRow being trait for BlockDeviceUsable would return the Header Row for
// BD returned from REST call with all set to false.
impl GetHeaderRow for BlockDeviceUsable {
    fn get_header_row(&self) -> Row {
        (&*BLOCKDEVICE_HEADERS_USABLE).clone()
    }
}

#[async_trait(?Send)]
impl GetBlockDevices for BlockDevice {
    type ID = NodeId;
    async fn get_blockdevices(id: &Self::ID, all: &bool, output: &OutputFormat) {
        let mut used_disks: Vec<String> = vec![];
        match RestClient::client().pools_api().get_node_pools(id).await {
            Ok(pools) => {
                for pool in pools.into_body() {
                    let mut normalized_disks: Vec<String> = pool
                        .spec
                        .unwrap()
                        .disks
                        .into_iter()
                        .map(|disk| normalize_disk(disk.as_str()))
                        .collect();
                    used_disks.append(&mut normalized_disks);
                }
            }
            Err(e) => {
                println!("Failed to list blockdevices for node {} . Error {}", id, e);
                return;
            }
        }

        match RestClient::client()
            .block_devices_api()
            .get_node_block_devices(id, Some(*all))
            .await
        {
            Ok(blockdevices) => {
                // Print table, json or yaml based on output format.
                if *all {
                    let bds: Vec<BlockDeviceAll> =
                        transform(&used_disks, blockdevices.into_body(), true)
                            .into_iter()
                            .map(BlockDeviceAll)
                            .collect();
                    print_table(output, bds);
                } else {
                    let bds: Vec<BlockDeviceUsable> =
                        transform(&used_disks, blockdevices.into_body(), false)
                            .into_iter()
                            .map(BlockDeviceUsable)
                            .collect();
                    print_table(output, bds);
                };
            }
            Err(e) => {
                println!("Failed to list blockdevices for node {} . Error {}", id, e)
            }
        }
    }
}

fn transform(
    used_disks: &[String],
    blockdevices: Vec<openapi::models::BlockDevice>,
    all: bool,
) -> Vec<openapi::models::BlockDevice> {
    let mut transformed_bds = Vec::with_capacity(blockdevices.len());
    for mut bd in blockdevices {
        let mut possible_values = bd.devlinks.clone();
        possible_values.push(bd.devname.clone());
        if bd.available && check_available(used_disks, possible_values) {
            bd.available = true;
            transformed_bds.push(bd);
        } else if all {
            bd.available = false;
            transformed_bds.push(bd);
        }
    }
    transformed_bds
}

fn check_available(used_disks: &[String], possible_values: Vec<String>) -> bool {
    for value in possible_values {
        if used_disks.contains(&value) {
            return false;
        }
    }
    true
}

fn normalize_disk(disk: &str) -> String {
    Url::parse(disk).map_or(disk.to_string(), |u| {
        u.to_file_path()
            .unwrap_or_else(|_| disk.into())
            .as_path()
            .display()
            .to_string()
    })
}
