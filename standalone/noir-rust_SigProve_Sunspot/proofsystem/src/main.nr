use eddsa::eddsa_verify;
use poseidon::poseidon::bn254;
use poseidon::poseidon::PoseidonHasher;

global OP_EQ: u32 = 0;
global OP_LT: u32 = 1;
global OP_LE: u32 = 2;
global OP_GT: u32 = 3;
global OP_GE: u32 = 4;
global OP_MAX: u32 = 4;

global ATTR_INT: u32  = 0;
global ATTR_BOOL: u32 = 2;

struct Constraint {
    pub operation:      u32,
    pub value:          u32,
    pub attribute_type: u32,
}

fn main(
    data:               [u32; 5],
    signature:          [u8; 96],

    constraints:        pub [Constraint; 5],
    creation_timestamp: pub u32,

    pub_key_x:          pub [u8; 32],
    pub_key_y:          pub [u8; 32],
) {
    for i in 0..5 {
        let d   = data[i];
        let op  = constraints[i].operation;
        let val = constraints[i].value;
        let t   = constraints[i].attribute_type;

        assert(op <= OP_MAX);
        assert((t == ATTR_INT) | (t == ATTR_BOOL));

        if t == ATTR_BOOL {
            assert(d * (d - 1) == 0);
            assert(val * (val - 1) == 0);
        }

        if op == OP_EQ { assert(d == val); }
        if op == OP_LT { assert(d <  val); }
        if op == OP_LE { assert(d <= val); }
        if op == OP_GT { assert(d >  val); }
        if op == OP_GE { assert(d >= val); }
    }

    let h0: Field = bn254::hash_2([data[0] as Field, data[1] as Field]);
    let h1: Field = bn254::hash_2([h0,               data[2] as Field]);
    let h2: Field = bn254::hash_2([h1,               data[3] as Field]);
    let h3: Field = bn254::hash_2([h2,               data[4] as Field]);
    let hashed_field: Field = bn254::hash_2([h3,     creation_timestamp as Field]);

    assert(eddsa_verify::<PoseidonHasher>(
        bytes32_to_field(pub_key_x),
        bytes32_to_field(pub_key_y),
        bytes32_to_field(signature_chunk(signature, 64)),
        bytes32_to_field(signature_chunk(signature, 0)),
        bytes32_to_field(signature_chunk(signature, 32)),
        hashed_field,
    ));
}

fn signature_chunk(signature: [u8; 96], offset: u32) -> [u8; 32] {
    let mut bytes = [0; 32];
    for i in 0..32 {
        bytes[i] = signature[offset + i];
    }
    bytes
}

fn bytes32_to_field(bytes: [u8; 32]) -> Field {
    let mut value: Field = 0;
    for i in 0..32 {
        value = value * 256 + bytes[i] as Field;
    }
    value
}
