<#
.SYNOPSIS
  Notifica a Esplora file la sostituzione delle icone di app e installer.
.DESCRIPTION
  Invalida la cache tramite Windows senza terminare Esplora file, cancellare
  i database su disco o modificare i collegamenti dell'utente.
  Eseguire dopo aver ricompilato l'installer, oppure passare i file aggiornati.
#>
[CmdletBinding()]
param([string[]]$Percorsi = @())

$ErrorActionPreference = 'Stop'
if (-not $Percorsi.Count) {
  $dist = Join-Path (Split-Path -Parent $PSScriptRoot) 'dist'
  $Percorsi = @(Join-Path $dist 'ITA-OCR-windows\ocr-ita-desktop.exe')
  $Percorsi += @(Get-ChildItem -LiteralPath $dist -Filter 'ITA-OCR-setup*.exe' |
    Select-Object -ExpandProperty FullName)
}

if (-not ('ItaOcr.ShellNotify' -as [type])) {
  Add-Type @'
using System.Runtime.InteropServices;
namespace ItaOcr {
  public static class ShellNotify {
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    public static extern void SHChangeNotify(uint evento, uint flags,
      string percorso, string secondoPercorso);
  }
}
'@
}

foreach ($percorso in $Percorsi) {
  $file = Get-Item -LiteralPath $percorso
  if ($file.PSIsContainer) { throw "Atteso un file: $percorso" }
  # SHCNE_UPDATEITEM, SHCNF_PATHW | SHCNF_FLUSH: usa il percorso reale e
  # aspetta la consegna della notifica prima di terminare.
  [ItaOcr.ShellNotify]::SHChangeNotify(0x2000, 0x1005, $file.FullName, $null)
  Write-Host "Notificato aggiornamento icona: $($file.FullName)"
}

# UPDATEITEM da solo puo' lasciare la vecchia immagine nelle finestre di
# Explorer anche se SHGetFileInfo, da un altro processo, restituisce gia'
# quella nuova. ASSOCCHANGED invalida anche la cache delle icone della shell.
# SHCNF_IDLIST | SHCNF_FLUSH, nessun percorso per questa notifica globale.
[ItaOcr.ShellNotify]::SHChangeNotify(0x08000000, 0x1000, $null, $null)
Write-Host 'Cache icone della shell invalidata.'
