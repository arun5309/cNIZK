use poseidon::poseidon::bn254;

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
    randomness:         [u8; 16],
    creation_timestamp: pub Field,
    constraints:        pub [Constraint; 5],
    message:            pub Field,
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

    let mut rand_field: Field = 0;
    for i in 0..16 {
        rand_field = rand_field * 256 + randomness[i] as Field;
    }

    let h0: Field = bn254::hash_2([data[0] as Field, data[1] as Field]);
    let h1: Field = bn254::hash_2([h0,               data[2] as Field]);
    let h2: Field = bn254::hash_2([h1,               data[3] as Field]);
    let h3: Field = bn254::hash_2([h2,               data[4] as Field]);
    let h4: Field = bn254::hash_2([h3,               creation_timestamp]);
    let computed: Field = bn254::hash_2([h4,          rand_field]);

    assert(computed == message);

}
