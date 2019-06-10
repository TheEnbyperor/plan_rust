global vga_println
global vga_print
global vga_print_char

section .text
bits 32
vga_println:
    push eax
    push ebx
    push ecx
    push edx

    call vga_print

    ; newline
    mov edx, 0
    mov eax, [vga_position]
    mov ecx, 80 * 2
    div ecx
    add eax, 1
    mul ecx
    mov [vga_position], eax

    pop edx
    pop ecx
    pop ebx
    pop eax

    ret

; print a string
; IN
;   esi: points at zero-terminated String
; CLOBBER
;   ah, ebx
vga_print:
    cld
vga_print_loop:
    ; note: if direction flag is set (via std)
    ; this will DECREMENT the ptr, effectively
    ; reading/printing in reverse.
    lodsb
    test al, al
    jz vga_print_done
    call vga_print_char
    jmp vga_print_loop
vga_print_done:
    ret


; print a character
; IN
;   al: character to print
; CLOBBER
;   ah, ebx
vga_print_char:
    mov ebx, [vga_position]
    mov ah, 0x0f
    mov word [ebx + 0xb8000], ax

    add ebx, 2
    mov [vga_position], ebx

    ret

vga_position:
	dd 0