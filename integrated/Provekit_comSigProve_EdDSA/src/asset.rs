use crate::constants::{ATTR_STRING, CONSTRAINT_COUNT, DATA_COUNT, OP_EQ};
use crate::types::{Constraint, TokenizedAsset};

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn hash_string_to_u64(value: &str) -> u64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in value.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

pub fn extract_asset_inputs(asset: &TokenizedAsset) -> Result<(Vec<u64>, Vec<Constraint>), String> {

    let value: u64 = asset
        .value
        .trim()
        .parse()
        .map_err(|_| format!("asset.value {:?} is not a valid non-negative integer", asset.value))?;

    let fields = [
        asset.version.as_str(),
        asset.id.as_str(),
        asset.value.as_str(),
        asset.unit.as_str(),
        asset.creation_timestamp.as_str(),
        asset.issuer_signature.as_str(),
        asset.issuer_address.as_str(),
        asset.custodian_address.as_str(),
        asset.owner_address.as_str(),
        asset.asset_type.as_str(),
        asset.serial_number.as_str(),
        asset.tag.as_str(),
        asset.parent_id.as_str(),
        asset.status.as_str(),
    ];

    let mut data = fields
        .iter()
        .map(|field| hash_string_to_u64(field))
        .collect::<Vec<_>>();
    data[2] = value;

    let constraints = data
        .iter()
        .map(|v| Constraint {
            operation: OP_EQ,
            value: *v,
            attribute_type: ATTR_STRING,
        })
        .collect::<Vec<_>>();

    debug_assert_eq!(data.len(), DATA_COUNT);
    debug_assert_eq!(constraints.len(), CONSTRAINT_COUNT);

    Ok((data, constraints))
}
