pub mod tss;

use x86_64::{
    instructions::{
        segmentation::{Segment, CS},
        tables::load_tss,
    },
    structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
};

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&tss::TSS));
        (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
            },
        )
    };
}

pub fn init() {
    GDT.0.load();

    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}
