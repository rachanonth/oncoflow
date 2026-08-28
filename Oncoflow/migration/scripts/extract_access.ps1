param(
  [Parameter(Mandatory = $true)]
  [string] $Source
)

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$OutputEncoding = [System.Text.UTF8Encoding]::new($false)

$resolvedSource = (Resolve-Path -LiteralPath $Source).Path
$connection = [System.Data.OleDb.OleDbConnection]::new(
  "Provider=Microsoft.ACE.OLEDB.16.0;Data Source=$resolvedSource;Mode=Read;"
)

function Convert-AccessValue {
  param([object] $Value)

  if ($Value -is [System.DBNull]) {
    return $null
  }
  if ($Value -is [datetime]) {
    return $Value.ToString('yyyy-MM-dd HH:mm:ss', [System.Globalization.CultureInfo]::InvariantCulture)
  }
  if ($Value -is [bool]) {
    return [bool] $Value
  }
  if (
    $Value -is [byte] -or $Value -is [int16] -or $Value -is [int32] -or
    $Value -is [int64]
  ) {
    return [int64] $Value
  }
  if (
    $Value -is [single] -or $Value -is [double] -or $Value -is [decimal]
  ) {
    return [double] $Value
  }
  return [string] $Value
}

try {
  $connection.Open()
  $tables = $connection.GetOleDbSchemaTable(
    [System.Data.OleDb.OleDbSchemaGuid]::Tables,
    @($null, $null, $null, 'TABLE')
  ) | Where-Object {
    $_.TABLE_NAME -notmatch '^MSys' -and $_.TABLE_NAME -notmatch '^dbo_'
  } | Sort-Object TABLE_NAME

  foreach ($table in $tables) {
    $tableName = [string] $table.TABLE_NAME
    $primaryKeys = @(
      $connection.GetOleDbSchemaTable(
        [System.Data.OleDb.OleDbSchemaGuid]::Primary_Keys,
        @($null, $null, $tableName)
      ) | Sort-Object ORDINAL | ForEach-Object { [string] $_.COLUMN_NAME }
    )
    $columnMetadata = @(
      $connection.GetOleDbSchemaTable(
        [System.Data.OleDb.OleDbSchemaGuid]::Columns,
        @($null, $null, $tableName, $null)
      ) | Sort-Object ORDINAL_POSITION | ForEach-Object {
        [pscustomobject] @{
          name = [string] $_.COLUMN_NAME
          accessType = ([System.Data.OleDb.OleDbType] ([int] $_.DATA_TYPE)).ToString()
          ordinal = [int] $_.ORDINAL_POSITION
          nullable = [bool] $_.IS_NULLABLE
          size = if ($_.CHARACTER_MAXIMUM_LENGTH -is [System.DBNull]) { $null } else { [int64] $_.CHARACTER_MAXIMUM_LENGTH }
        }
      }
    )
    $escapedTable = $tableName.Replace(']', ']]')
    $command = $connection.CreateCommand()
    $command.CommandText = "SELECT * FROM [$escapedTable]"
    $reader = $command.ExecuteReader()
    $rows = [System.Collections.Generic.List[object]]::new()
    try {
      while ($reader.Read()) {
        $row = [ordered] @{}
        for ($index = 0; $index -lt $reader.FieldCount; $index++) {
          $columnName = $reader.GetName($index)
          if ($tableName -eq 'TblUser' -and $columnName -ieq 'password') {
            continue
          }
          $row[$columnName] = Convert-AccessValue -Value $reader.GetValue($index)
        }
        $rows.Add([pscustomobject] $row)
      }
    }
    finally {
      $reader.Dispose()
      $command.Dispose()
    }

    [pscustomobject] @{
      name = $tableName
      rowCount = $rows.Count
      primaryKeys = $primaryKeys
      columns = $columnMetadata
      rows = $rows
    } | ConvertTo-Json -Compress -Depth 6
  }
}
finally {
  $connection.Dispose()
}
