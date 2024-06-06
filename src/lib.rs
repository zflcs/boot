
extern crate alloc;
extern crate proc_macro;
use alloc::vec::Vec;
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{parse_macro_input, Ident, ItemFn};

#[proc_macro_attribute]
pub fn riscv_entry(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr_string = attr.to_string();
    let boot_stack_attr: Vec<&str> = attr_string.split(':').collect();
    let boot_stack_ident = Ident::new(&boot_stack_attr[0].to_ascii_uppercase(), Span::call_site());
    let without_prefix = boot_stack_attr[1].trim().trim_start_matches("0x");
    let boot_stack_size = usize::from_str_radix(without_prefix, 16).unwrap();
    let input_fn = parse_macro_input!(item as ItemFn);
    let derive_macro: TokenStream = quote!(
        #[link_section = ".bss.stack"]
        static mut #boot_stack_ident: [u8; #boot_stack_size] = [0; #boot_stack_size];
        /// Entry of kernel.
        #[naked]
        #[no_mangle]
        #[link_section = ".text.entry"]
        unsafe extern "C" fn __entry(hartid: usize) -> ! {
            core::arch::asm!(
                // Use tp to save hartid
                "mv tp, a0",
                // Set stack pointer to the kernel stack.
                "
                la a1, {stack}
                li t0, {total_stack_size}
                li t1, {stack_size}
                mul sp, a0, t1
                sub sp, t0, sp
                add sp, a1, sp
                call {clear_bss}
                ",        // Jump to the main function.
                "j  {main}",
                total_stack_size = const #boot_stack_size,
                stack_size       = const #boot_stack_size,
                stack            =   sym BOOT_STACK,
                main             =   sym main,
                clear_bss        =   sym clear_bss,
                options(noreturn),
            )
        }
        use core::panic::PanicInfo;

        #[panic_handler]
        fn panic(info: &PanicInfo) -> ! {
            log::error!("{}, {}", info.location().unwrap(), info.message().unwrap());
            loop {}
        }
        pub fn clear_bss() {
            extern "C" {
                fn s_bss();
                fn e_bss();
            }
            (s_bss as usize..e_bss as usize).for_each(|a| unsafe { (a as *mut u8).write_volatile(0) });
        }
        #[no_mangle]
        #input_fn
    ).into();
    // println!("{}", derive_macro);
    derive_macro
}
