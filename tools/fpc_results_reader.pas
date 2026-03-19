{ fpc_results_reader.pas — Read RESULTS.DAT using Borland Pascal conventions.
  Compile: fpc fpc_results_reader.pas
  Usage:   ./fpc_results_reader <RESULTS.DAT path>

  Reads the file as `file of TResultsRecord` (84-byte records) and prints
  each record's kind, chain pointers, text (via String[72] length prefix),
  and a hex dump of the raw text area to help diagnose boundary detection
  differences between oracle and Rust-generated files. }

program fpc_results_reader;

{$mode objfpc}
{$H-}  { Use short strings (Borland-compatible) }

uses SysUtils;

type
  TResultsTail = array[0..9] of Byte;
  TResultsRecord = packed record
    Kind: Byte;
    Text: String[72];   { 1 byte length + 72 chars = 73 bytes }
    Tail: TResultsTail;  { 10 bytes }
  end;
  { Total: 1 + 73 + 10 = 84 bytes }

const
  END_OF_TRANSMISSION = '<end of transmission>';

var
  F: file of TResultsRecord;
  Rec: TResultsRecord;
  RecIdx: Integer;
  ChainId, NextChainId: Word;
  TextLen: Byte;
  TextStr: String;
  i: Integer;
  ByteAfterLen: Byte;
  HasNonZeroTrail: Boolean;
  FilePath: String;

begin
  if ParamCount < 1 then
  begin
    WriteLn('Usage: fpc_results_reader <RESULTS.DAT>');
    Halt(1);
  end;
  FilePath := ParamStr(1);

  WriteLn('Record size check: SizeOf(TResultsRecord) = ', SizeOf(TResultsRecord));

  Assign(F, FilePath);
  {$I-}
  Reset(F);
  {$I+}
  if IOResult <> 0 then
  begin
    WriteLn('Error: cannot open ', FilePath);
    Halt(1);
  end;

  WriteLn('File: ', FilePath);
  WriteLn('File size: ', FileSize(F), ' records (', FileSize(F) * SizeOf(TResultsRecord), ' bytes)');
  WriteLn;

  RecIdx := 0;
  while not Eof(F) do
  begin
    Read(F, Rec);
    TextLen := Length(Rec.Text);
    TextStr := Rec.Text;

    { Extract chain pointers from tail }
    ChainId := Rec.Tail[0] or (Rec.Tail[1] shl 8);
    NextChainId := Rec.Tail[4] or (Rec.Tail[5] shl 8);

    { Check if there are non-zero bytes after the string length in the text area }
    HasNonZeroTrail := False;
    for i := TextLen + 1 to 72 do
    begin
      { Rec.Text[0] is length byte, Rec.Text[1..72] are char data.
        We want to check chars after the declared length. }
      if Ord(Rec.Text[i]) <> 0 then
      begin
        HasNonZeroTrail := True;
        Break;
      end;
    end;

    { Byte immediately after the text content }
    if TextLen < 72 then
      ByteAfterLen := Ord(Rec.Text[TextLen + 1])
    else
      ByteAfterLen := 0;

    WriteLn(Format('--- Record %d ---', [RecIdx]));
    WriteLn(Format('  Kind:         $%02x', [Rec.Kind]));
    WriteLn(Format('  TextLen:      %d', [TextLen]));
    WriteLn(Format('  Text:         "%s"', [TextStr]));
    WriteLn(Format('  ChainId:      %d', [ChainId]));
    WriteLn(Format('  NextChainId:  %d', [NextChainId]));
    WriteLn(Format('  ByteAfterLen: $%02x', [ByteAfterLen]));
    WriteLn(Format('  NonZeroTrail: %s', [BoolToStr(HasNonZeroTrail, True)]));

    { Hex dump of the text area bytes (positions 1..72 of the string) }
    Write('  TextHex:      ');
    for i := 1 to 72 do
    begin
      Write(Format('%02x', [Ord(Rec.Text[i])]));
      if (i mod 16 = 0) and (i < 72) then
        Write(LineEnding + '                ')
      else if i < 72 then
        Write(' ');
    end;
    WriteLn;

    { Flag end-of-transmission records }
    if TextStr = END_OF_TRANSMISSION then
      WriteLn('  ** END OF TRANSMISSION **');

    { Flag boundary: header record (ChainId > 0 or RecIdx = 0) with NextChainId }
    if (RecIdx = 0) or (ChainId > 0) then
    begin
      if NextChainId > 0 then
        WriteLn(Format('  >> HEADER: chain %d -> next %d (next report starts at record %d)',
          [ChainId, NextChainId, NextChainId - 1]))
      else
        WriteLn(Format('  >> HEADER: chain %d -> LAST REPORT', [ChainId]));
    end;

    WriteLn;
    Inc(RecIdx);
  end;

  Close(F);
  WriteLn(Format('Total: %d records read.', [RecIdx]));
end.
