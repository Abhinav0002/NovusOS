ENTRY(_start)

SECTIONS
{
    . = 0x40080000;

    .text.boot : {
        KEEP(*(.text.boot))
    }

    .text : {
        *(.text .text.*)
    }

    . = ALIGN(4096);
    .rodata : {
        *(.rodata .rodata.*)
    }

    . = ALIGN(4096);
    .data : {
        *(.data .data.*)
    }

    . = ALIGN(4096);
    __bss_start = .;
    .bss : {
        *(.bss .bss.*)
        *(COMMON)
    }
    __bss_end = .;

    . = ALIGN(4096);
    __kernel_end = .;

    . = . + 0x10000;
    . = ALIGN(16);
    __stack_top = .;
}
