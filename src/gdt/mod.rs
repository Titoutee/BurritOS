use core::ptr::addr_of;
use lazy_static::lazy_static;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));
        (
            gdt,
            Selectors {
                code_selector,
                tss_selector,
            },
        )
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

pub fn init() {
    use x86_64::instructions::segmentation::{Segment, CS};
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();
    // Suppose the selectors are valid
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
}

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

// The Interrupt Stack in the case of a Double Fault.
// Double faults indeed need a separate stack, switched from the user task one, because:
// In the case of user-stack overflow, the double fault immediately propagates to a triple fault (fatal),
// as the double fault's ISF will also reside in the guard page.
// Thus, all double faults are handled and no triple fault can occur
lazy_static! {
    /// !!!CAREFUL!!!: The Interrupt Stack Frame should not be host to stack-intensive tasks
    /// as NO guard page is present
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            const STACK_SIZE: usize = 4096 * 5;
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            let stack_start = VirtAddr::from_ptr(unsafe { addr_of!(STACK) }); // Addr_of
            let stack_end = stack_start + STACK_SIZE as u64;
            stack_end
        };
        tss
    };
}
