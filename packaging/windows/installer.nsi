Unicode true
!include "MUI2.nsh"
Name "OpenPDF Desktop (Development)"
OutFile "../../dist/OpenPDF-Desktop-0.1.0-windows-x64-setup.exe"
InstallDir "$LOCALAPPDATA\OpenPDF Desktop"
RequestExecutionLevel user
Icon "../../assets/icon.ico"
UninstallIcon "../../assets/icon.ico"
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_LICENSE "../../LICENSE"
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
Section "Application"
 SetOutPath "$INSTDIR"
 File /r "../../dist/windows-x64/*"
 WriteUninstaller "$INSTDIR\Uninstall.exe"
 CreateDirectory "$SMPROGRAMS\OpenPDF Desktop"
 CreateShortcut "$SMPROGRAMS\OpenPDF Desktop\OpenPDF Desktop.lnk" "$INSTDIR\openpdf.exe"
 CreateShortcut "$SMPROGRAMS\OpenPDF Desktop\Uninstall.lnk" "$INSTDIR\Uninstall.exe"
 WriteRegStr HKCU "Software\Classes\OpenPDF.Document" "" "PDF document"
 WriteRegStr HKCU "Software\Classes\OpenPDF.Document\DefaultIcon" "" "$INSTDIR\icon.ico"
 WriteRegStr HKCU "Software\Classes\OpenPDF.Document\shell\open\command" "" '$\"$INSTDIR\openpdf.exe$\" $\"%1$\"'
 WriteRegStr HKCU "Software\Classes\.pdf\OpenWithProgids" "OpenPDF.Document" ""
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\openpdf.exe" "" "$INSTDIR\openpdf.exe"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\OpenPDFDesktop" "DisplayName" "OpenPDF Desktop (Development)"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\OpenPDFDesktop" "UninstallString" '$\"$INSTDIR\Uninstall.exe$\"'
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\OpenPDFDesktop" "DisplayVersion" "0.1.0"
 WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\OpenPDFDesktop" "Publisher" "OpenPDF contributors"
SectionEnd
Section "Uninstall"
 !include "../../dist/windows-uninstall.nsh"
 Delete "$INSTDIR\Uninstall.exe"
 RMDir "$INSTDIR"
 Delete "$SMPROGRAMS\OpenPDF Desktop\OpenPDF Desktop.lnk"
 Delete "$SMPROGRAMS\OpenPDF Desktop\Uninstall.lnk"
 RMDir "$SMPROGRAMS\OpenPDF Desktop"
 DeleteRegValue HKCU "Software\Classes\.pdf\OpenWithProgids" "OpenPDF.Document"
 DeleteRegKey HKCU "Software\Classes\OpenPDF.Document"
 DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\App Paths\openpdf.exe"
 DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\OpenPDFDesktop"
 ; Deliberately retain the profile, encrypted identity, and user documents.
SectionEnd
