.code64
.section    .entry, "awx"

.global     entry, entry_ret
entry:                                  # pub extern "sysv64" fn entry(boot_args: *const BootArgs);
    call    set_vmm_tss64
    call    save_uefi_regs
    call    init_vmm_regs
    lea     vmm_gdtr(%rip), %rax
    lgdt    (%rax)
    mov     $8, %rax
    shl     $3, %rax
    ltr     %ax
    mov     $10, %rax
    shl     $3, %rax
    lldt    %ax
    mov     %rsp, uefi_rsp(%rip)
    lea     vmm_stack_end(%rip), %rax
    mov     %rax, %rsp
    lea     vmm_main(%rip), %rax
    mov     %rax, vmm_main_ljmp(%rip)
    mov     $4, %ax                     # DATA64
    shl     $3, %ax
    mov     %ax, %ds
    mov     %ax, %es
    mov     %ax, %ss
    mov     %ax, %fs
    mov     %ax, %gs
    lea     vmm_main_ljmp(%rip), %rax
    .byte   0x48                        # REX.W prefix
    ljmpl   *(%rax)
entry_ret:
    hlt
    mov     uefi_rsp(%rip), %rsp
    call    restore_uefi_regs
    ret

.align      16
vmm_main_ljmp:
    .quad   0
    .quad   0x18                        # CODE64

.global     save_uefi_regs
save_uefi_regs:                         # 1st arg: uefi_cr3
    push    %rax
    push    %rcx
    push    %rdx
    mov     %cs, uefi_cs(%rip)
    mov     %ds, uefi_ds(%rip)
    mov     %es, uefi_es(%rip)
    mov     %fs, uefi_fs(%rip)
    mov     %gs, uefi_gs(%rip)
    mov     %ss, uefi_ss(%rip)
    lea     uefi_gdtr(%rip), %rax
    sgdt    (%rax)
    lea     uefi_idtr(%rip), %rax
    sidt    (%rax)
    lea     uefi_ldtr(%rip), %rax
    sldt    (%rax)
    lea     uefi_tr(%rip), %rax
    str     (%rax)
    mov     %cr0, %rax
    mov     %rax, uefi_cr0(%rip)
    mov     (%rdi), %rax
    mov     %rax, uefi_cr3(%rip)
    mov     %cr4, %rax
    mov     %rax, uefi_cr4(%rip)
    mov     $0x174, %rcx                # MSR_IA32_SYSENTER_CS
    rdmsr
    mov     %ax, uefi_msr_ia32_sysenter_cs(%rip)
    mov     $0x175, %rcx                # MSR_IA32_SYSENTER_ESP
    rdmsr
    mov     %eax, uefi_msr_ia32_sysenter_esp(%rip)
    mov     %edx, uefi_msr_ia32_sysenter_esp_high(%rip)
    mov     $0x176, %rcx                # MSR_IA32_SYSENTER_EIP
    rdmsr
    mov     %eax, uefi_msr_ia32_sysenter_eip(%rip)
    mov     %edx, uefi_msr_ia32_sysenter_eip_high(%rip)
    pop     %rdx
    pop     %rcx
    pop     %rax
    ret

.global     restore_uefi_regs
restore_uefi_regs:
    push    %rax
    push    %rcx
    push    %rdx
    lea     uefi_gdtr(%rip), %rax
    lgdt    (%rax)
    lea     uefi_idtr(%rip), %rax
    lidt    (%rax)
    lea     uefi_ldtr(%rip), %rax
    lldt    (%rax)
    xor     %rax, %rax
    mov     uefi_cs(%rip), %ax
    push    %rax
    lea     1f(%rip), %rax
    push    %rax
    lretq
1:  mov     uefi_ds(%rip), %ds
    mov     uefi_es(%rip), %es
    mov     uefi_fs(%rip), %fs
    mov     uefi_gs(%rip), %gs
    mov     uefi_ss(%rip), %ss
    # hold cr0, cr4 state!!!
#    mov     uefi_cr4(%rip), %rax
#    or      $0b10000000000000, %rax     # VMXE bit(intel only, FIXME)
#    mov     %rax, %cr4
#    mov     uefi_cr0(%rip), %rax
#    mov     %rax, %cr0
    xor     %rax, %rax
    mov     $0x174, %rcx                # MSR_IA32_SYSENTER_CS
    mov     uefi_msr_ia32_sysenter_cs(%rip), %eax
    wrmsr
    xor     %rax, %rax
    mov     $0x175, %rcx                # MSR_IA32_SYSENTER_ESP
    mov     uefi_msr_ia32_sysenter_esp(%rip), %eax
    mov     uefi_msr_ia32_sysenter_esp_high(%rip), %rdx
    wrmsr
    xor     %rax, %rax
    mov     $0x176, %rcx                # MSR_IA32_SYSENTER_EIP
    mov     uefi_msr_ia32_sysenter_eip(%rip), %eax
    mov     uefi_msr_ia32_sysenter_eip_high(%rip), %rdx
    wrmsr
    pop     %rdx
    pop     %rcx
    pop     %rax
    ret

.global     init_vmm_regs
init_vmm_regs:
    push    %rax
    push    %rbx
    mov     $(vmm_gdt_end - vmm_gdt), %ax   # sizeof GDT
    mov     %ax, vmm_gdtr(%rip)
    lea     vmm_gdtr(%rip), %rax
    lea     vmm_gdt(%rip), %rbx
    mov     %rbx, 2(%rax)
    mov     %cr3, %rax
    mov     %rax, vmm_cr3(%rip)
    mov     %cr4, %rax
    or      $(0x20|0x80|0x2000), %rax   # PAE + PGE + VMXE
    and     $(~0x40), %rax              # !MCE
    mov     %rax, vmm_cr4(%rip)
    mov     %rax, %cr4
    pop     %rbx
    pop     %rax
    ret

.global     set_vmm_tss64
set_vmm_tss64:
    push    %rax
    push    %rcx
    push    %rdx
    xor     %rcx, %rcx
    lea     vmm_tss64(%rip), %rax
    lea     vmm_gdt(%rip), %rdx
    mov     %ax, %cx
    mov     %cx, 0x42(%rdx)             # base address 15:00
    shr     $16, %rax
    mov     %ax, %cx
    mov     %cl, 0x44(%rdx)             # base address 23:16
    mov     %ch, 0x47(%rdx)             # base address 31:24
    shr     $16, %rax
    mov     %eax, %ecx
    mov     %ecx, 0x48(%rdx)            # base address 63:32
    # movb    $0x89, 0x45(%rdx)           # TSS-available
    pop     %rdx
    pop     %rcx
    pop     %rax
    ret

# ===== UEFI special registers =====
.align      2
.global     uefi_cs
uefi_cs:
    .short  0

.align      2
.global     uefi_ds
uefi_ds:
    .short  0

.align      2
.global     uefi_es
uefi_es:
    .short  0

.align      2
.global     uefi_fs
uefi_fs:
    .short  0

.align      2
.global     uefi_gs
uefi_gs:
    .short  0

.align      2
.global     uefi_ss
uefi_ss:
    .short  0

.align      8
.global     uefi_cr0
uefi_cr0:
    .quad   0

.align      8
.global     uefi_cr3
uefi_cr3:
    .quad   0

.align      8
.global     uefi_cr4
uefi_cr4:
    .quad   0

.align      8
.global     uefi_rsp
uefi_rsp:
    .quad   0

.align      16
.global     uefi_gdtr
uefi_gdtr:
    .space  16

.align      16
.global     uefi_idtr
uefi_idtr:
    .space  16

.align      2
.global     uefi_ldtr
uefi_ldtr:
    .short  0

.align      2
.global     uefi_tr
uefi_tr:
    .short  0

.align      2
.global     uefi_msr_ia32_sysenter_cs
uefi_msr_ia32_sysenter_cs:
    .short  0

.align      16
.global     uefi_msr_ia32_sysenter_esp
uefi_msr_ia32_sysenter_esp:
    .word   0
.global     uefi_msr_ia32_sysenter_esp_high
uefi_msr_ia32_sysenter_esp_high:
    .word   0

.align      16
.global     uefi_msr_ia32_sysenter_eip
uefi_msr_ia32_sysenter_eip:
    .word   0
.global     uefi_msr_ia32_sysenter_eip_high
uefi_msr_ia32_sysenter_eip_high:
    .word   0
# === UEFI special registers end ===

# ===== VMM special registers =====
.align      8
.global     vmm_cr3
vmm_cr3:
    .quad   0

.align      8
.global     vmm_cr4
vmm_cr4:
    .quad   0

.align      16
.global     vmm_gdtr
vmm_gdtr:
    .short  0
    .quad   0

.align      16
.global     vmm_gdt, vmm_gdt_end
vmm_gdt:
    .quad   0x0000000000000000          # NULL
    .quad   0x00CF9B000000FFFF          # 0x08 CODE32, DPL=0
    .quad   0x00CF93000000FFFF          # 0x10 DATA32, DPL=0
    .quad   0x00AF9B000000FFFF          # 0x18 CODE64, DPL=0
    .quad   0x00AF93000000FFFF          # 0x20 DATA64, DPL=0
    .quad   0x00009B000000FFFF          # 0x28 CODE16, DPL=0
    .quad   0x000093000000FFFF          # 0x30 DATA16, DPL=0
    .quad   0x0000930B8000FFFF          # 0x38 DATA16, DPL=0
    .quad   0x000089000000FFFF          # 0x40 TSS64(LOW)
    .quad   0x0000000000000000          # 0x48 TSS64(HIGH)
    .quad   0x000082000000FFFF          # 0x50 LDT(LOW)
    .quad   0x0000000000000000          # 0x58 LDT(HIGH)
vmm_gdt_end:

.align      16
.global     vmm_tss64
vmm_tss64:
    .word   0x00000000                  # reserved
    .quad   0x0000000000000000          # RSP0
    .quad   0x0000000000000000          # RSP1
    .quad   0x0000000000000000          # RSP2
    .quad   0x0000000000000000          # reserved
    .quad   0x0000000000000000          # IST1
    .quad   0x0000000000000000          # IST2
    .quad   0x0000000000000000          # IST3
    .quad   0x0000000000000000          # IST4
    .quad   0x0000000000000000          # IST5
    .quad   0x0000000000000000          # IST6
    .quad   0x0000000000000000          # IST7
    .quad   0x0000000000000000          # reserved
    .short  0x0000                      # reserved
    .short  0xffff                      # iomap base
# === VMM special registers end ===

# ===== VMM stack =====
.align      1024
.global     vmm_stack
vmm_stack:
    .space  0x1000*32
.global     vmm_stack_end
vmm_stack_end:
# === VMM stack end ===
