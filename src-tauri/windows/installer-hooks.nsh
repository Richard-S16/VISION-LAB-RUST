!macro NSIS_HOOK_PREINSTALL
vision_lab_check_running:
  nsExec::ExecToStack 'cmd /C tasklist /FI "IMAGENAME eq vision-lab-tauri.exe" /NH | findstr /I /B /C:"vision-lab-tauri.exe" >NUL'
  Pop $0
  Pop $1
  StrCmp $0 "0" 0 vision_lab_preinstall_done

  MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION \
    "VISION/LAB is currently running.$\r$\n$\r$\nClose the application, then click Retry to continue the upgrade." \
    IDRETRY vision_lab_check_running IDCANCEL vision_lab_cancel_install

vision_lab_cancel_install:
  Quit

vision_lab_preinstall_done:
!macroend
