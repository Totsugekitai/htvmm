.code64
.section    .entry

.global     entry 
entry: # pub extern "sysv64" fn entry(boot_args: *const BootArgs);
    call    *save_uefi_regs(%rip)
    call    *create_page_table(%rip)
    cli
    lea     pml4e(%rip), %rax
    mov     %rax, %cr3
    call    *vmm_main(%rip)
    ret

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

# create 4GiB identity mapping
.global     create_page_table
create_page_table:
    mov     $0x7, %rax                  # user access + writable + present
    lea     pdpe(%rip), %rbx
    or      %rax, %rbx
    mov     %rbx, pml4e(%rip)           # pml4e[0]
    lea     pde0(%rip), %rbx
    or      %rax, %rbx
    mov     %rbx, pdpe(%rip)            # pdpe[0]
    lea     pde1(%rip), %rbx
    or      %rax, %rbx
    mov     %rbx, pdpe(%rip)            # pdpe[1]
    lea     pde2(%rip), %rbx
    or      %rax, %rbx
    mov     %rbx, pdpe(%rip)            # pdpe[2]
    lea     pde3(%rip), %rbx
    or      %rax, %rbx
    mov     %rbx, pdpe(%rip)            # pdpe[3]
    mov     $0x83, %rbx                 # pagesize=2MiB + writable + present
    xor     %ecx, %ecx
0:  mov     %rbx, pde0(,%ecx,8)         # pde0[0] ~ pde0[511]
    add     $0x200000, %rbx             #
    add     $1, %ecx                    #
    cmp     $512, %ecx                  #
    jb      0b                          #
    xor     %ecx, %ecx
1:  mov     %rbx, pde1(,%ecx,8)         # pde1[0] ~ pde1[511]
    add     $0x200000, %rbx             #
    add     $1, %ecx                    #
    cmp     $512, %ecx                  #
    jb      1b                          #
    xor     %ecx, %ecx
2:  mov     %rbx, pde2(,%ecx,8)         # pde2[0] ~ pde2[511]
    add     $0x200000, %rbx             #
    add     $1, %ecx                    #
    cmp     $512, %ecx                  #
    jb      2b                          #
    xor     %ecx, %ecx
3:  mov     %rbx, pde3(,%ecx,8)         # pde3[0] ~ pde3[511]
    add     $0x200000, %rbx             #
    add     $1, %ecx                    #
    cmp     $512, %ecx                  #
    jb      3b                          #
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

# ===== VMM page table =====
.align      0x1000
.global     pml4e
pml4e:
    .space  8*512

.align      0x1000
.global     pdpe
pdpe:
    .space  8*512

.align      0x1000
.global     pde0
pde0:
    .space  8*512

.align      0x1000
.global     pde1
pde1:
    .space  8*512

.align      0x1000
.global     pde2
pde2:
    .space  8*512

.align      0x1000
.global     pde3
pde3:
    .space  8*512
# === VMM page table end ===
