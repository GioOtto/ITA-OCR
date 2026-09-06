; Installer di ITA-OCR per Windows (Inno Setup 6).
;
; Perche' Inno e non NSIS: il pacchetto non compresso e' 2,28 GB e NSIS si
; ferma a 2 GB con un "error mmapping file ... is out of range". Non e'
; aggirabile con opzioni, e' il suo limite. Inno legge i file in streaming e
; regge payload di queste dimensioni.
;
; Perche' un installer e non solo la cartella portabile: un self-extracting
; non si firma in modo affidabile (Authenticode mette la firma in coda al PE,
; dove l'SFX tiene i dati dell'archivio), non lascia un disinstallatore e non
; permette di scegliere i componenti.
;
; L'installazione e' **per utente**, in %LOCALAPPDATA%: non chiede
; l'amministratore e non tocca niente di condiviso. E' la stessa promessa
; della cartella portabile, con in piu' collegamenti e disinstallazione.
;
; Si compila cosi' (SORGENTE e' la cartella portabile gia' costruita):
;   ISCC /DSORGENTE=...\dist\ITA-OCR-windows /DUSCITA=...\dist installer.iss

#ifndef SORGENTE
  #error "manca /DSORGENTE=<cartella portabile>"
#endif
#ifndef USCITA
  #define USCITA "."
#endif
#ifndef VERSIONE
  #define VERSIONE "1.0.0"
#endif

[Setup]
AppId={{7C1E4F2A-9B3D-4E58-A6C7-0D1F2E3A4B5C}
AppName=ITA-OCR
AppVersion={#VERSIONE}
AppPublisher=Giorgio Ottoboni
VersionInfoCompany=Giorgio Ottoboni
VersionInfoCopyright=Giorgio Ottoboni
VersionInfoDescription=Installazione di ITA-OCR
VersionInfoVersion={#VERSIONE}

; Installazione per utente: niente UAC, niente scritture fuori dal profilo.
PrivilegesRequired=lowest
DefaultDirName={localappdata}\Programs\ITA-OCR
DefaultGroupName=ITA-OCR
DisableProgramGroupPage=yes
AllowNoIcons=yes
UsePreviousSetupType=no

; I modelli sono GGUF gia' quantizzati e si comprimono poco; la resa viene
; dalle DLL, cublasLt da sola e' 638 MB.
Compression=lzma2/max
SolidCompression=yes
LZMANumBlockThreads=4

ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

OutputDir={#USCITA}
OutputBaseFilename=ITA-OCR-setup_v{#VERSIONE}
SetupIconFile={#SORGENTE}\..\..\app\src-tauri\icons\icon.ico
WizardStyle=modern
UninstallDisplayIcon={app}\ocr-ita-desktop.exe
UninstallDisplayName=ITA-OCR

[Languages]
Name: "italiano"; MessagesFile: "compiler:Languages\Italian.isl"

[Types]
Name: "completa";    Description: "Installazione completa"
Name: "leggera";     Description: "Senza i modelli"
Name: "scelta";      Description: "Scelta manuale"; Flags: iscustom

[Components]
Name: "app"; Description: "Applicazione, motore e dizionari"; \
  Types: completa leggera scelta; Flags: fixed
Name: "modelli"; Description: "Modelli GLM-OCR (1,2 GB)"; \
  Types: completa scelta

[Tasks]
Name: "desktop"; Description: "Crea un collegamento sul desktop"; \
  GroupDescription: "Collegamenti"; Flags: unchecked

[Files]
; Radice: l'applicazione, PDFium e il runtime C++ accanto all'eseguibile,
; cosi' non serve il Visual C++ Redistributable sulla macchina.
Source: "{#SORGENTE}\ocr-ita-desktop.exe"; DestDir: "{app}"; Components: app; Flags: ignoreversion
Source: "{#SORGENTE}\pdfium.dll";          DestDir: "{app}"; Components: app; Flags: ignoreversion
Source: "{#SORGENTE}\ITA-OCR.cmd";         DestDir: "{app}"; Components: app; Flags: ignoreversion
Source: "{#SORGENTE}\msvcp140.dll";        DestDir: "{app}"; Components: app; Flags: ignoreversion
Source: "{#SORGENTE}\vcruntime140.dll";    DestDir: "{app}"; Components: app; Flags: ignoreversion
Source: "{#SORGENTE}\vcruntime140_1.dll";  DestDir: "{app}"; Components: app; Flags: ignoreversion

; Il motore ha una copia sua del runtime C++: il caricatore cerca accanto
; all'eseguibile, e llama-server.exe sta qui dentro.
Source: "{#SORGENTE}\llama\*";        DestDir: "{app}\llama";        Components: app; Flags: ignoreversion recursesubdirs
Source: "{#SORGENTE}\dictionaries\*"; DestDir: "{app}\dictionaries"; Components: app; Flags: ignoreversion recursesubdirs

; Senza questo componente l'applicazione cerca i modelli nella cartella
; indicata da OCR_ITA_MODELS.
Source: "{#SORGENTE}\models\*"; DestDir: "{app}\models"; Components: modelli; Flags: ignoreversion recursesubdirs

[Icons]
Name: "{group}\ITA-OCR";              Filename: "{app}\ocr-ita-desktop.exe"
Name: "{group}\Disinstalla ITA-OCR";  Filename: "{uninstallexe}"
Name: "{autodesktop}\ITA-OCR";        Filename: "{app}\ocr-ita-desktop.exe"; Tasks: desktop

[Run]
Filename: "{app}\ocr-ita-desktop.exe"; Description: "Avvia ITA-OCR"; \
  Flags: nowait postinstall skipifsilent

[UninstallDelete]
; Le cartelle restano se contengono file che non abbiamo messo noi: cosi' un
; modello aggiunto a mano dall'utente non sparisce senza preavviso.
Type: dirifempty; Name: "{app}\models"
Type: dirifempty; Name: "{app}\llama"
Type: dirifempty; Name: "{app}\dictionaries"
Type: dirifempty; Name: "{app}"

; Log, impostazioni e sessioni restano in %APPDATA%\ocr-ita-desktop: sono dati
; dell'utente, e cancellarli in silenzio sarebbe una sorpresa sgradita.
