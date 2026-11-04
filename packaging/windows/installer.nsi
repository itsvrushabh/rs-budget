; NSIS Installer script for Actual Budget (Windows)
!define APP_NAME "Actual Budget"
!define APP_VERSION "0.1.0"
!define APP_PUBLISHER "Actual Budget Contributors"
!define APP_EXE "actual-desktop.exe"

Name "${APP_NAME}"
OutFile "actual-budget-setup.exe"
InstallDir "$PROGRAMFILES64\Actual Budget"
RequestExecutionLevel admin

Section "MainSection" SEC01
    SetOutPath "$INSTDIR"
    File "target\release\actual-desktop.exe"
    CreateDirectory "$SMPROGRAMS\Actual Budget"
    CreateShortCut "$SMPROGRAMS\Actual Budget\Actual Budget.lnk" "$INSTDIR\actual-desktop.exe"
    CreateShortCut "$DESKTOP\Actual Budget.lnk" "$INSTDIR\actual-desktop.exe"
SectionEnd
