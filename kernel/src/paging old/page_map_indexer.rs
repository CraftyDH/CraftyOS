#[derive(Debug)]
pub struct PageMapIndexer {
    pub p_i: u64,
    pub pt_i: u64,
    pub pd_i: u64,
    pub pdp_i: u64,
}

impl PageMapIndexer {
    pub fn new(v_addr: u64) -> PageMapIndexer {
        // Magic stuff
        let mut addr = v_addr >> 12;
        let p_i = addr & 0x1ff;
        addr >>= 9;
        let pt_i = addr & 0x1ff;
        addr >>= 9;
        let pd_i = addr & 0x1ff;
        addr >>= 9;
        let pdp_i = addr & 0x1ff;

        PageMapIndexer {
            p_i,
            pt_i,
            pd_i,
            pdp_i,
        }
    }
}
