if exist \EFI\BOOT\BootX64.efi then
 \EFI\BOOT\BootX64.efi
 goto END
endif

if exist fs0:\EFI\BOOT\BootX64.efi then
 fs0:\EFI\BOOT\BootX64.efi
 goto END
endif

:END
