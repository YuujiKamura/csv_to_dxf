# CSV to DXF Road Section Converter

This tool automatically generates DXF files of road cross-sections from tabular data (CSV or clipboard). It allows for easy conversion of survey data or road design data into CAD drawings.

## Key Features

- Import measurement point data from clipboard and convert to DXF format
- Automatic rendering of road cross-sections
- Automatic placement of station point, distance, and width information
- Clear drawing layout
- Support for one-sided width data (left-only or right-only)
- Section name automatically used as default filename

## Recent Updates

### 2025-04-30: Windows Executable (EXE) Added

- Added Windows executable (EXE) file
- Standalone application that runs without Python environment
- Executable available at `dist/csv_to_dxf.exe`

### 2025-04-29: DXF Saving Function Update

- Fixed parameter mismatch in `CSVProcessor.save_dxf()` method
- Added more robust error logging

## Project Structure

- `src/` - Source code files
- `test/` - Test files
- `data/` - Sample data and test data
- `gui/` - GUI application modules
  - `app_controller.py` - Application controller
  - `ui/` - UI components
  - `core/` - Core functionality modules
- `dist/csv_to_dxf/` - Built Windows application

## Requirements

### For Python Environment

- Python 3.6 or higher
- Required Python libraries:
  - pandas (for data processing)
  - ezdxf (for DXF creation)
  - PyQt6 (for GUI interface)

### For Windows Users

- No Python installation required when using the pre-built executable
- Just download the repository and run the EXE file

## Setup

### Python Environment Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/csv_to_dxf.git
cd csv_to_dxf

# Install required libraries
pip install pandas ezdxf PyQt6
```

### Windows Direct Execution

1. Clone or download the GitHub repository
2. Run `dist/csv_to_dxf/csv_to_dxf.exe`

## Usage

### GUI Application

1. Start the application:

```bash
# Using Python
python main.py

# Or on Windows, using the executable
dist/csv_to_dxf/csv_to_dxf.exe
```

2. Click the "Browse..." button to select a CSV file
3. Select the desired section from the dropdown menu
4. Click the "Preview" button to preview the data
5. Click the "Convert to DXF" button to generate the DXF file
   - The file save dialog will automatically populate with the selected section name as the default filename

## Screenshots

Application preview screen:

![Application Preview](screenshots/app_preview.png)

### Command-line Operation

1. Create road section data using Excel or other spreadsheet software
2. Copy the data (Ctrl+C)
3. Run the application with the following command

```bash
python src/table_to_dxf.py
```

Or use the provided batch file:

```bash
run_converter.bat
```

### Input Data Format

The input data should consist of the following 4 columns:

| Station Name | Distance (m) | Left Width (m) | Right Width (m) |
|--------------|--------------|----------------|-----------------|
| No.0         | 0            | 3.50           | 3.35            |
| No.1         | 10           | 3.40           | 3.35            |
| ...          | ...          | ...            | ...             |

**Notes:**
- Column headers are not required
- Station names should be entered as text (e.g., No.0, No.1, etc.)
- Distance and width values should be numeric
- Decimal values are processed accurately
- **For one-sided width data, enter 0 for the width on the other side**

### Input Examples

Standard example:
```
No.0    0       3.50    3.35
10m     10      3.40    3.35
No.1    20      3.45    3.40
```

Left-side width only example:
```
No.0    0       3.50    0.00
10m     10      3.40    0.00
No.1    20      3.45    0.00
```

Right-side width only example:
```
No.0    0       0.00    3.35
10m     10      0.00    3.40
No.1    20      0.00    3.50
```

## Output File

After running the program, a DXF file will be generated in the selected directory.
This file can be opened with the following software:

- AutoCAD
- Jw_cad
- LibreCAD
- Other CAD software that supports DXF

## Running Tests

To run the automated tests, use the following commands:

```bash
# Run all tests
python -m unittest discover -s test

# Run individual tests
python -m unittest test.test_loader
python -m unittest test.test_table_to_dxf
python -m unittest test.test_dxf_draw_tenkaiz
python -m unittest test.test_dxf_draw_tenkaiz_one_side  # Tests for one-sided width
```

## Common Errors and Solutions

### Error: "Could not load data from clipboard"

**Solution:**
- Verify that data has been copied to clipboard correctly
- Check that the data format has 4 columns
- Make sure to select the cell range accurately in the spreadsheet before copying

### Error: "The first column (station name) must be a string"

**Solution:**
- Check that the station name (1st column) is a string
- If using only numbers, add text before or after (e.g., "No." + number)

### Error: "DXF file is not created"

**Solution:**
- Check the console output for detailed error messages
- Review the input data format
- Make sure existing DXF files are not open (which may prevent overwriting)

## License

This software is released under the GNU General Public License (GPL) version 3.

**Note**: This application uses PyQt6, which is provided under the GPL license. As a result, this entire application (both source code and binary) is subject to the terms of the GPL. GPL is a copyleft license, which means that any distributed derivative works must also be distributed under the same license terms (GPL) with source code availability. 