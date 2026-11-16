//! 6×6 Expert puzzles.
//!
//! A 6×6 puzzle that needs a naked or hidden pair is rare: about one dig in
//! five thousand, which is most of a second of search on a desktop and several
//! on a phone. So this one level is not searched for on demand. The bank below
//! holds 64 puzzles the search found (each rated Expert and checked to have one
//! solution, see `tests/difficulty.rs`), and a seed picks one and re-arranges
//! it: digits are relabelled, rows are permuted inside and across the three
//! bands, columns inside and across the two stacks. Those moves keep every
//! row, column and box a row, column and box, so the puzzle needs exactly the
//! same techniques, while giving about 2.5 million forms per bank puzzle.

/// Zero is an empty cell. Regenerate with `search_rated_puzzle(6, seed, Expert)`.
const BANK: [&str; 64] = [
    "005000010000000002300640040300600400",
    "000064000100104000320000003400002003",
    "000204000000100000006300050010004002",
    "000400050010600000532000000000100203",
    "035000400000000060600024501000000002",
    "000405000021002000650002060000010006",
    "106500300010000042000000400300060000",
    "000500000010000040030006064002002000",
    "043000600000005040001600004000200001",
    "000004000500400002001000060300030060",
    "003000600005206100030002000001004200",
    "000000000052100000053000300400460010",
    "000400500010002000400060010003200000",
    "200040034000000560000001006210002006",
    "050100010004000600005020003000040500",
    "201060000000000000604010000020060540",
    "000060004052460000005000100000000203",
    "100000050010360000000000510034030560",
    "024000600000000560001000000020030004",
    "000502000041002400300200000000610000",
    "000405001000203000014000000600030002",
    "240060500000000000000601003500000010",
    "200000040000060000005010003056000003",
    "000000200603000300001046010000005004",
    "063001004000046030000000000050000203",
    "004000600030005100000020006200400001",
    "000000050214000105000000040000016050",
    "200000006000100050050003000020000541",
    "061500530000000604000001020000610400",
    "040000501000630004005300000600100020",
    "001000000030000000204003050600002004",
    "005001406000000305000020001030060000",
    "001000400001003060006300000050050004",
    "100000532000000030000500000040200001",
    "200000506100400003000040000006020500",
    "160000000000400000031200000034005102",
    "000000430020000400006030200005010000",
    "013620000000250000001000020160000030",
    "060410001000000230000004200000053000",
    "100000003210005400000060402000000300",
    "063000250000040036000002000200000054",
    "000005506000000020020051003000010040",
    "103000260000002160000400000600001005",
    "030000000001005600004002050006600200",
    "001060000100006000350000400003500040",
    "501000030100000000000026003400400005",
    "000010004005020600600200300000005000",
    "200000000030000004650000000020060041",
    "004000000002060000200503000000016300",
    "000000250010030200000004006040000500",
    "000005530004006040020006640003000000",
    "510000030000004030000402000001000260",
    "000004000620003500040002006000010005",
    "001050605000100060000002300010000004",
    "006020000410304000020000000000260001",
    "020050010600050000006001003405000000",
    "500600060300000002030500040000001003",
    "001005500030105000060000000600002004",
    "041060000000000000053040005000100203",
    "000600006020000003300010050000010405",
    "000630000000002000500000006023205001",
    "003504000006004601050000000000016405",
    "500000240000000230000006300500060002",
    "103000050000006100000002000004302050",
];

#[doc(hidden)]
pub fn expert_6x6_bank() -> &'static [&'static str] {
    &BANK
}

/// A Fisher–Yates shuffle on the generator's high bits; the low bits of this
/// kind of generator repeat too quickly for two- and three-item shuffles.
fn permute<T>(items: &mut [T], state: &mut u32) {
    for index in (1..items.len()).rev() {
        *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        items.swap(index, (*state >> 16) as usize % (index + 1));
    }
}

/// Lines `0..count*group` shuffled inside each group and with the groups shuffled.
fn lines(count: usize, group: usize, state: &mut u32) -> Vec<usize> {
    let mut inside: Vec<usize> = (0..count * group).collect();
    for part in inside.chunks_mut(group) {
        permute(part, state);
    }
    let mut groups: Vec<usize> = (0..count).collect();
    permute(&mut groups, state);
    groups
        .iter()
        .flat_map(|&g| inside[g * group..(g + 1) * group].iter().copied())
        .collect()
}

pub(crate) fn puzzle(seed: u32) -> Vec<u8> {
    let mut state = crate::difficulty::attempt_seed(seed, 0);
    let base = BANK[(state >> 16) as usize % BANK.len()].as_bytes();
    let mut digits: Vec<u8> = (1..=6).collect();
    permute(&mut digits, &mut state);
    let rows = lines(3, 2, &mut state);
    let columns = lines(2, 3, &mut state);
    rows.iter()
        .flat_map(|&row| {
            let digits = &digits;
            columns
                .iter()
                .map(move |&column| match base[row * 6 + column] - b'0' {
                    0 => 0,
                    value => digits[value as usize - 1],
                })
        })
        .collect()
}
