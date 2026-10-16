param([switch]$Child, [switch]$ValidateOnly, [string]$FixtureRoot,
    [string]$ReceiptPath, [string]$SourceRevision)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'windows-launch-diagnostic.ps1')
# CUR-260: restricted own token, not a new Windows account. No actual upgrade claim.
$nativeSource = @'
using System;
using System.Text;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Security.Principal;
public static class BiankRestrictedRunner {
 [StructLayout(LayoutKind.Sequential)] struct SidAttr { public IntPtr Sid; public uint Attributes; }
 [StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)] struct Startup {
  public int cb; public string reserved,desktop,title; public uint x,y,cx,cy,charsX,charsY,fill,flags;
  public ushort show,reserved2; public IntPtr reservedPtr,input,output,error;
 }
 [StructLayout(LayoutKind.Sequential)] struct ProcessInfo { public IntPtr process,thread; public uint pid,tid; }
 [StructLayout(LayoutKind.Sequential)] struct BasicLimit {
  public long processTime,jobTime; public uint flags; public UIntPtr min,max; public uint active;
  public UIntPtr affinity; public uint priority,scheduling;
 }
 [StructLayout(LayoutKind.Sequential)] struct Io { public ulong r,w,o,rb,wb,ob; }
 [StructLayout(LayoutKind.Sequential)] struct ExtendedLimit {
  public BasicLimit basic; public Io io; public UIntPtr processMemory,jobMemory,peakProcess,peakJob;
 }
 [DllImport("kernel32.dll")] static extern IntPtr GetCurrentProcess();
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool CloseHandle(IntPtr h);
 [DllImport("advapi32.dll",SetLastError=true)] static extern bool OpenProcessToken(IntPtr p,uint access,out IntPtr t);
 [DllImport("advapi32.dll",SetLastError=true)] static extern bool CreateRestrictedToken(IntPtr t,uint flags,uint count,[In] SidAttr[] sids,uint pc,IntPtr ps,uint sc,IntPtr ss,out IntPtr result);
 [DllImport("advapi32.dll",SetLastError=true)] static extern bool GetTokenInformation(IntPtr t,int kind,IntPtr data,int size,out int needed);
 [DllImport("advapi32.dll",SetLastError=true)] static extern bool CheckTokenMembership(IntPtr t,IntPtr sid,out bool member);
 [DllImport("advapi32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern bool LookupPrivilegeValue(string system,string name,out long luid);
 [DllImport("advapi32.dll",SetLastError=true)] static extern bool SetTokenInformation(IntPtr t,int kind,IntPtr data,int size);
 [DllImport("advapi32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern bool CreateProcessAsUser(IntPtr t,string app,StringBuilder args,IntPtr pa,IntPtr ta,bool inherit,uint flags,IntPtr env,string cwd,ref Startup startup,out ProcessInfo info);
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern IntPtr CreateJobObject(IntPtr attrs,string name);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool SetInformationJobObject(IntPtr job,int kind,ref ExtendedLimit limits,int size);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool AssignProcessToJobObject(IntPtr job,IntPtr process);
 [DllImport("kernel32.dll",SetLastError=true)] static extern uint ResumeThread(IntPtr thread);
 [DllImport("kernel32.dll",SetLastError=true)] static extern uint WaitForSingleObject(IntPtr h,uint ms);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool GetExitCodeProcess(IntPtr p,out uint code);
 [DllImport("kernel32.dll",SetLastError=true)] static extern bool TerminateProcess(IntPtr p,uint code);
 static void Need(bool ok) { if(!ok) throw new InvalidOperationException("Win32_"+Marshal.GetLastWin32Error()); }
 static IntPtr Sid(SecurityIdentifier id) {
  byte[] bytes=new byte[id.BinaryLength];id.GetBinaryForm(bytes,0);
  IntPtr p=Marshal.AllocHGlobal(bytes.Length);Marshal.Copy(bytes,0,p,bytes.Length);return p;
 }
 static IntPtr Info(IntPtr token,int kind) {
  int length;GetTokenInformation(token,kind,IntPtr.Zero,0,out length);Need(length>0);
  IntPtr p=Marshal.AllocHGlobal(length);
  try {Need(GetTokenInformation(token,kind,p,length,out length));return p;}
  catch {Marshal.FreeHGlobal(p);throw;}
 }
 public static Dictionary<string,bool> Inspect(string expectedSid) {
  IntPtr token=IntPtr.Zero,admin=IntPtr.Zero,groups=IntPtr.Zero,privileges=IntPtr.Zero;
  try {
   Need(OpenProcessToken(GetCurrentProcess(),8,out token));
   admin=Sid(new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid,null));
   bool effective;Need(CheckTokenMembership(IntPtr.Zero,admin,out effective));
   groups=Info(token,2);int count=Marshal.ReadInt32(groups);bool deny=false;
   int offset=IntPtr.Size, stride=Marshal.SizeOf<SidAttr>();
   for(int i=0;i<count;i++) {
    var entry=Marshal.PtrToStructure<SidAttr>(IntPtr.Add(groups,offset+i*stride));
    if(new SecurityIdentifier(entry.Sid).IsWellKnown(WellKnownSidType.BuiltinAdministratorsSid))
     deny=(entry.Attributes&0x10)!=0 && (entry.Attributes&4)==0;
   }
   long debug;Need(LookupPrivilegeValue(null,"SeDebugPrivilege",out debug));
   privileges=Info(token,3);bool present=false;
   for(int i=0;i<Marshal.ReadInt32(privileges);i++) if(Marshal.ReadInt64(privileges,4+12*i)==debug) present=true;
   return new Dictionary<string,bool>{{"effective_admin",effective},{"admin_sid_deny_only",deny},
    {"sedebug_present",present},{"own_sid_preserved",WindowsIdentity.GetCurrent().User.Value==expectedSid}};
  } finally {
   if(privileges!=IntPtr.Zero)Marshal.FreeHGlobal(privileges);
   if(groups!=IntPtr.Zero)Marshal.FreeHGlobal(groups);
   if(admin!=IntPtr.Zero)Marshal.FreeHGlobal(admin);
   if(token!=IntPtr.Zero)CloseHandle(token);
  }
 }
 public static int Run(string executable,string args,string cwd,string environment) {
  IntPtr original=IntPtr.Zero,restricted=IntPtr.Zero,admin=IntPtr.Zero,env=IntPtr.Zero,job=IntPtr.Zero;
  ProcessInfo process=new ProcessInfo();
  try {
   Need(OpenProcessToken(GetCurrentProcess(),0x000B,out original));
   admin=Sid(new SecurityIdentifier(WellKnownSidType.BuiltinAdministratorsSid,null));
   // DISABLE_MAX_PRIVILEGE; disable Administrators only, retain caller's user SID.
   Need(CreateRestrictedToken(original,1,1,new[]{new SidAttr{Sid=admin}},0,IntPtr.Zero,0,IntPtr.Zero,out restricted));
   env=Marshal.StringToHGlobalUni(environment);
   job=CreateJobObject(IntPtr.Zero,null);Need(job!=IntPtr.Zero);
   var limits=new ExtendedLimit();limits.basic.flags=0x2000; // KILL_ON_JOB_CLOSE
   Need(SetInformationJobObject(job,9,ref limits,Marshal.SizeOf<ExtendedLimit>()));
   var startup=new Startup();startup.cb=Marshal.SizeOf<Startup>();
   Need(CreateProcessAsUser(restricted,executable,new StringBuilder("\""+executable+"\" "+args),
    IntPtr.Zero,IntPtr.Zero,false,0x404,env,cwd,ref startup,out process));
   // Suspended child joins its owned job before it can create any grandchildren.
   Need(AssignProcessToJobObject(job,process.process));Need(ResumeThread(process.thread)!=0xffffffff);
   uint wait=WaitForSingleObject(process.process,240000);
   if(wait!=0)throw new InvalidOperationException("Restricted_child_timeout_or_wait_failure");
   uint code;Need(GetExitCodeProcess(process.process,out code));return checked((int)code);
  } finally {
   // Only this freshly created child/job is owned. Never terminate by name or PID search.
   if(process.process!=IntPtr.Zero && WaitForSingleObject(process.process,0)!=0) {
    TerminateProcess(process.process,1);WaitForSingleObject(process.process,10000);
   }
   if(job!=IntPtr.Zero)CloseHandle(job);
   if(process.thread!=IntPtr.Zero)CloseHandle(process.thread);
   if(process.process!=IntPtr.Zero)CloseHandle(process.process);
   if(env!=IntPtr.Zero)Marshal.FreeHGlobal(env);
   if(admin!=IntPtr.Zero)Marshal.FreeHGlobal(admin);
   if(restricted!=IntPtr.Zero)CloseHandle(restricted);
   if(original!=IntPtr.Zero)CloseHandle(original);
  }
 }
}
'@
Add-Type -TypeDefinition $nativeSource
if ($ValidateOnly) {
    Write-Output 'STATIC_COMPILE_ONLY: C# compiled; Windows native assertions NOT_RUN'
    return
}
if (!$IsWindows) { throw 'Windows native execution required; no skip or Linux PASS' }
$repo = Split-Path $PSScriptRoot -Parent
if ($Child) {
    $receipt = [ordered]@{ schema=1; verdict='FAIL'; scope='restricted-token installer guards, not actual upgrade';
        source_revision=$env:BIANK_QA_SOURCE_REVISION; native_assertions='RUNNING'; stage='token-proof' }
    try {
        $proof = [BiankRestrictedRunner]::Inspect($env:BIANK_QA_EXPECTED_SID)
        $receipt.token = $proof
        if ($proof.effective_admin -or !$proof.admin_sid_deny_only -or $proof.sedebug_present -or !$proof.own_sid_preserved) {
            throw 'Restricted token proof failed'
        }
        $receipt.stage = 'fixture-write'
        $probe = Join-Path $FixtureRoot 'writable-proof.txt'
        [IO.File]::WriteAllText($probe,'CUR260 synthetic fixture')
        if ([IO.File]::ReadAllText($probe) -cne 'CUR260 synthetic fixture') { throw 'Fixture write failed' }
        $receipt.fixture_write = $true
        $receipt.stage = 'dependency-nsis'
        if (!(Test-Path 'C:/Program Files (x86)/NSIS/makensis.exe')) { throw 'NSIS must be prepared by parent' }
        $receipt.nsis_available = $true
        foreach ($name in @('windows-installer-hook.test.ps1')) {
            $receipt.stage = 'dependency-nsis-harness'
            if (!(Test-Path (Join-Path $PSScriptRoot $name))) { throw 'Missing existing harness' }
        }
        $receipt.harnesses_available = $true
        $receipt.stage = 'dependency-node-resolution'
        $nodeCommand = Get-Command node -ErrorAction Stop
        $node = $nodeCommand.Source
        $receipt.node_resolved = $true
        $receipt.node_command_is_application = $nodeCommand.CommandType -eq 'Application'
        $receipt.node_path_is_file = [bool]($node -and (Test-Path -LiteralPath $node -PathType Leaf))
        $receipt.stage = 'dependency-node-execution'
        $nodeVersion = & $node --version 2>$null
        $receipt.node_exit_code = $LASTEXITCODE
        $receipt.stage = 'dependency-node-version'
        if ($LASTEXITCODE -ne 0 -or $nodeVersion -notmatch '^v22\.') { throw 'Node22 required' }
        $receipt.node22_verified = $true
        Set-Location $repo
        $receipt.stage = 'existing-nsis-hook'
        & (Join-Path $PSScriptRoot 'windows-installer-hook.test.ps1') *> $null
        $receipt.nsis_hook = $true
        $receipt.native_assertions = 'PASS'; $receipt.verdict='PASS_RESTRICTED_TOKEN_GUARDS'; $receipt.stage='complete'
    } catch {
        if ($receipt.stage -eq 'dependency-node-execution') {
            $receipt.node_failure = Get-Cur260LaunchFailure -Record $_
            $receipt.direct_launch_probe = Get-Cur260DirectLaunchProbe -FilePath $node
            $receipt.node_read_probe = Get-Cur260FileReadProbe -FilePath $node
            # The executable of this running restricted child, not a PATH lookup.
            # --version only, same launcher options, no elevation or token change.
            $control = [System.Environment]::ProcessPath
            $receipt.control_read_probe = Get-Cur260FileReadProbe -FilePath $control
            $receipt.control_launch_probe = Get-Cur260DirectLaunchProbe -FilePath $control
        }
        # Deliberately retain fixed stage only: never exception text, paths, SIDs or stdout.
        $receipt.failure_bucket = $receipt.stage
    } finally {
        $receipt | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $FixtureRoot 'child-receipt.json') -Encoding utf8
    }
    if ($receipt.verdict -ne 'PASS_RESTRICTED_TOKEN_GUARDS') { exit 1 }; exit 0
}
if (!$ReceiptPath) { throw 'ReceiptPath required' }
if (!$SourceRevision -or $SourceRevision -notmatch '^[0-9a-f]{40}$') { throw 'Full source SHA required' }
if (Test-Path -LiteralPath $ReceiptPath) { throw 'Receipt must not overwrite existing evidence' }
$fixture = Join-Path ([IO.Path]::GetTempPath()) ('biank-standard-user-' + [guid]::NewGuid())
$result = [ordered]@{schema=1;verdict='FAIL';source_revision=$SourceRevision;native_assertions='NOT_RUN';stage='parent-prepare';
    scope='restricted own SID; not a separate standard account or actual 0.3.8-to-0.3.9 upgrade'}
$exitCode = 1
try {
    New-Item -ItemType Directory -Path $fixture | Out-Null
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    # Give only the caller's actual SID and SYSTEM access; no administrator-group dependency.
    $acl = [Security.AccessControl.DirectorySecurity]::new()
    $acl.SetAccessRuleProtection($true,$false)
    foreach ($sid in @($identity.User,[Security.Principal.SecurityIdentifier]::new('S-1-5-18'))) {
        $rule = [Security.AccessControl.FileSystemAccessRule]::new($sid,'FullControl','ContainerInherit,ObjectInherit','None','Allow')
        $acl.AddAccessRule($rule)
    }
    Set-Acl -LiteralPath $fixture -AclObject $acl
    $environment = [ordered]@{PATH=$env:PATH;SystemRoot=$env:SystemRoot;WINDIR=$env:WINDIR;COMSPEC=$env:COMSPEC;
        PATHEXT=$env:PATHEXT;TEMP=$fixture;TMP=$fixture;RUNNER_TEMP=$fixture;
        BIANK_QA_EXPECTED_SID=$identity.User.Value;BIANK_QA_SOURCE_REVISION=$SourceRevision}
    # No inherited CI/service tokens, credentials, personal HOME or PS profiles.
    $block = (($environment.GetEnumerator() | Sort-Object Key | ForEach-Object { '{0}={1}' -f $_.Key,$_.Value }) -join "`0") + "`0`0"
    $exe = (Get-Process -Id $PID).Path
    $arguments = '-NoLogo -NoProfile -NonInteractive -File "{0}" -Child -FixtureRoot "{1}"' -f $PSCommandPath,$fixture
    $result.stage='restricted-launch'
    $exitCode = [BiankRestrictedRunner]::Run($exe,$arguments,$repo,$block)
    $result.child_exit_code = $exitCode
    $childReceipt = Join-Path $fixture 'child-receipt.json'
    if (!(Test-Path $childReceipt)) { throw 'Child receipt missing' }
    $result = Get-Content -Raw -LiteralPath $childReceipt | ConvertFrom-Json -AsHashtable
    $result.child_exit_code = $exitCode
    if ($exitCode -ne 0 -or $result.verdict -ne 'PASS_RESTRICTED_TOKEN_GUARDS') { throw 'Child assertions failed' }
} catch {
    $exitCode=1; $result.verdict='FAIL'
    if (!$result.Contains('failure_bucket')) { $result.failure_bucket=$result.stage }
} finally {
    try {
        Remove-Item -LiteralPath $fixture -Recurse -Force -ErrorAction Stop
        $result.owned_fixture_removed=$true
        $result.owned_job_closed=$true; $result.token_handles_closed=$true
    } catch { $exitCode=1; $result.verdict='FAIL'; $result.failure_bucket='cleanup' }
    $parent = Split-Path ([IO.Path]::GetFullPath($ReceiptPath)) -Parent
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
    $result | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $ReceiptPath -Encoding utf8
    Write-Output ('CUR260 receipt verdict: ' + $result.verdict)
}
if ($exitCode -ne 0) { throw 'CUR260 restricted-token assertions failed; see sanitized receipt' }
