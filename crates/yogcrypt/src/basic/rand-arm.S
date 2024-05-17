/* An implementation of the rand function for ARM */

.macro DECLARE_LOCAL_FUNC name
    .globl \name
    .hidden \name
    .type \name, %function
\name:
.endm

/* 
 * -------------------------------------
 * extern "C" uint32_t do_rdrand(uint32_t *rand);
 * return value:
 *    non-zero: rdrand succeeded
 *    zero: rdrand failed
 * -------------------------------------
 */
DECLARE_LOCAL_FUNC do_rdrand
    mrs x0, id_aa64isar0_el1
    ubfx x0, x0, #4, #4
    cbz x0, .Lno_rndr

    mov x1, #10
.Lrdrand_retry:
    rndr x0
    b.eq .Lrdrand_return
    subs x1, x1, #1
    b.ne .Lrdrand_retry
    mov x0, #0
    ret

.Lno_rndr:
    mov x0, #0
    ret

.Lrdrand_return:
    str w0, [x0]
    mov x0, #1
    ret