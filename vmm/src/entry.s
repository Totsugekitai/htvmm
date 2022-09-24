.code64
.section    .entry

.global     entry 
entry: # pub extern "sysv64" fn entry(boot_args: *const BootArgs);
    call    *save_uefi_regs(%rip)


.global     save_uefi_regs
save_uefi_regs:
    push    %rax
    push    %rcx
    push    %rdx
    mov     %cs, uefi_cs(%rip)
    mov     %ds, uefi_ds(%rip)
    mov     %es, uefi_es(%rip)
    mov     %fs, uefi_fs(%rip)
    mov     %gs, uefi_gs(%rip)
    mov     %ss, uefi_ss(%rip)
    sgdt    uefi_gdtr(%rip)
    sidt    uefi_idtr(%rip)
    sldt    uefi_ldtr(%rip)
    str     uefi_tr(%rip)
    mov     %cr3, %rax
    mov     %rax, uefi_cr3(%rip)
    mov     $0x174, %rcx        # MSR_IA32_SYSENTER_CS
    rdmsr
    mov     %ax, uefi_msr_ia32_sysenter_cs(%rip)
    mov     $0x175, %rcx        # MSR_IA32_SYSENTER_ESP
    rdmsr
    mov     %eax, uefi_msr_ia32_sysenter_esp(%rip)
    mov     %edx, uefi_msr_ia32_sysenter_esp_high(%rip)
    mov     $0x176, %rcx        # MSR_IA32_SYSENTER_EIP
    rdmsr
    mov     %eax, uefi_msr_ia32_sysenter_eip(%rip)
    mov     %edx, uefi_msr_ia32_sysenter_eip_high(%rip)
    pop     %rdx
    pop     %rcx
    pop     %rax
    ret


# ===== UEFI special registers store space =====
.global     uefi_cs
uefi_cs:
    .short  0

.global     uefi_ds
uefi_ds:
    .short  0

.global     uefi_es
uefi_es:
    .short  0

.global     uefi_fs
uefi_fs:
    .short  0

.global     uefi_gs
uefi_gs:
    .short  0

.global     uefi_ss
uefi_ss:
    .short  0

.global     uefi_cr3
uefi_cr3:
    .quad   0

.global     uefi_rsp
uefi_rsp:
    .quad   0

.global     uefi_gdtr
uefi_gdtr:
    .space  16

.global     uefi_idtr
uefi_idtr:
    .space  16

.global     uefi_ldtr
uefi_ldtr:
    .short  0

.global     uefi_tr
uefi_tr:
    .short  0

.global     uefi_msr_ia32_sysenter_cs
uefi_msr_ia32_sysenter_cs:
    .short  0

.global     uefi_msr_ia32_sysenter_esp
uefi_msr_ia32_sysenter_esp:
    .word   0
.global     uefi_msr_ia32_sysenter_esp_high
uefi_msr_ia32_sysenter_esp_high:
    .word   0

.global     uefi_msr_ia32_sysenter_eip
uefi_msr_ia32_sysenter_eip:
    .word   0
.global     uefi_msr_ia32_sysenter_eip_high
uefi_msr_ia32_sysenter_eip_high:
    .word   0
# === UEFI special registers store space end ===
