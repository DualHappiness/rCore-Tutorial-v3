.altmacro
.macro SAVE_SN n
    sd s\n, (\n+1)*8(sp)
.endm
.macro LOAD_SN n
    ld s\n, (\n+1)*8(sp)
.endm
    .section .text
    .globl __switch
__switch:
    # __switch(current: &*const TaskContent, next: &*const TaskContent)
    addi sp, sp, -13*8 # 分配TaskContent内存
    sd sp, 0(a0) # 将当前栈地址保存到current指针的位置
    # 保存各个寄存器
    sd ra, 0(sp) 
    .set n, 0
    .rept 12
        SAVE_SN %n
        .set n, n + 1
    .endr

    ld sp, 0(a1)
    ld ra, 0(sp)
    .set n, 0
    .rept 12
        LOAD_SN %n
        .set n, n + 1
    .endr
    addi sp, sp, 13*8
    ret
