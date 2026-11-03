use poseidon::poseidon::bn254;

global OP_EQ: u32 = 0;
global OP_NE: u32 = 1;
global OP_LT: u32 = 2;
global OP_LE: u32 = 3;
global OP_GT: u32 = 4;
global OP_GE: u32 = 5;
global OP_MAX: u32 = 5;

global ATTR_STRING: u32 = 1;
global ATTR_BOOL: u32   = 2;
global LOCKED_STATUS_HASH: u64 = 13301589205220017395;

struct Constraint {
    pub operation:      u32,
    pub value:          u64,
    pub attribute_type: u32,
}

fn main(
    data:               [u64; 14],
    randomness:         [u8; 16],
    creation_timestamp: pub Field,
    loan_amount:        pub u64,
    constraints:        pub [Constraint; 14],
    message:            pub Field,
) {
    for i in 0..14 {
        let d   = data[i];
        let op  = constraints[i].operation;
        let val = constraints[i].value;
        let t   = constraints[i].attribute_type;

        assert(op <= OP_MAX);

        if t == ATTR_STRING {
            assert((op == OP_EQ) | (op == OP_NE));
        }
        if t == ATTR_BOOL {
            assert(d * (d - 1) == 0);
            assert(val * (val - 1) == 0);
        }

        if op == OP_EQ { assert(d == val); }
        if op == OP_NE { assert(d != val); }
        if op == OP_LT { assert(d <  val); }
        if op == OP_LE { assert(d <= val); }
        if op == OP_GT { assert(d >  val); }
        if op == OP_GE { assert(d >= val); }
    }

    assert(data[13] == LOCKED_STATUS_HASH);

    assert(data[2] >= loan_amount);

    let mut rand_field: Field = 0;
    for i in 0..16 {
        rand_field = rand_field * 256 + randomness[i] as Field;
    }

    let h0: Field = bn254::hash_2([data[0] as Field, data[1] as Field]);
    let h1: Field = bn254::hash_2([h0,               data[2] as Field]);
    let h2: Field = bn254::hash_2([h1,               data[3] as Field]);
    let h3: Field = bn254::hash_2([h2,               data[4] as Field]);
    let h4: Field = bn254::hash_2([h3,               data[5] as Field]);
    let h5: Field = bn254::hash_2([h4,               data[6] as Field]);
    let h6: Field = bn254::hash_2([h5,               data[7] as Field]);
    let h7: Field = bn254::hash_2([h6,               data[8] as Field]);
    let h8: Field = bn254::hash_2([h7,               data[9] as Field]);
    let h9: Field = bn254::hash_2([h8,               data[10] as Field]);
    let h10: Field = bn254::hash_2([h9,              data[11] as Field]);
    let h11: Field = bn254::hash_2([h10,             data[12] as Field]);
    let h12: Field = bn254::hash_2([h11,             data[13] as Field]);
    let h13: Field = bn254::hash_2([h12,             creation_timestamp]);
    let h14: Field = bn254::hash_2([h13,             loan_amount as Field]);
    let computed: Field = bn254::hash_2([h14,        rand_field]);

    assert(computed == message);
}
