/****************************************************************************
**
** Copyright (C) 1992-2005 Trolltech AS. All rights reserved.
**
** This file is part of the painting module of the Qt Toolkit.
**
** This file may be distributed and/or modified under the terms of the
** GNU General Public License version 2 as published by the Free Software
** Foundation and appearing in the file LICENSE.GPL included in the
** packaging of this file.
**
** See http://www.trolltech.com/pricing.html or email sales@trolltech.com for
** information about Qt Commercial License Agreements.
** See http://www.trolltech.com/gpl/ for GPL licensing information.
**
** Contact info@trolltech.com if any conditions of this licensing are
** not clear to you.
**
** This file is provided AS IS with NO WARRANTY OF ANY KIND, INCLUDING THE
** WARRANTY OF DESIGN, MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE.
**
****************************************************************************/

#ifndef QT_NO_PRINTER

#include "qpsprinter_p.h"
#include "qpsprinter.h"
#include "qprintengine.h"
#include "qlist.h"
#include <qprintdialog.h>
#include <qpagesetupdialog.h>

#include <qprintengine_ps_p.h>

#ifdef QT3_SUPPORT
#  include "qprintdialog.h"
#endif // QT3_SUPPORT

#define ABORT_IF_ACTIVE(location) \
    if (d->printEngine->printerState() == QPSPrinter::Active) { \
        qWarning("%s, cannot be changed while printer is active", location); \
        return; \
    }

/*!
  \class QPSPrinter qpsprinter.h
  \brief The QPSPrinter class is a paint device that paints on a printer.

  \ingroup multimedia
  \mainclass

  On Windows or Mac OS X, QPSPrinter uses the built-in printer drivers. On X11, QPSPrinter
  generates postscript and sends that to lpr, lp, or another printProgram(). QPSPrinter
  can also print to any other QPrintEngine.

  QPSPrinter is used in much the same way as QWidget and QPixmap are
  used. The big difference is that you must keep track of pages.

  QPSPrinter supports a number of settable parameters, most of which can be
  changed by the end user through a \l{QAbstractPrintDialog} print dialog. In
  general, QPSPrinter passes these functions onto the underlying QPrintEngine.

  The most important parameters are:
  \list
  \i setOrientation() tells QPSPrinter which page orientation to use.
  \i setPageSize() tells QPSPrinter what page size to expect from the
  printer.
  \i setResolution() tells QPSPrinter what resolution you wish the
  printer to provide (in dots per inch -- dpi).
  \i setFullPage() tells QPSPrinter whether you want to deal with the
  full page or just with the part the printer can draw on. The
  default is false, so that by default you should be able to paint
  on (0,0). If true the origin of the coordinate system will be in
  the top left corner of the paper and most probably the printer
  will not be able to paint something there due to it's physical
  margins.
  \i setNumCopies() tells QPSPrinter how many copies of the document
  it should print.
  \endlist

  Many of the settable functions can only be called before the actual printing
  begins (i.e., before QPainter::begin() is called). This usually makes sense
  (e.g., you can't change the number of copies when you are halfway through
  printing). There are also some settings that the user sets (through the
  printer dialog) and that applications are expected to obey. See
  QAbstractPrintDialog's documentation for more details.

  Once you start printing, calling newPage() is essential. You will
  probably also need to look at the device metrics for the
  printer.

  If you want to abort the print job, abort() will try its best to
  stop printing. It may cancel the entire job or just part of it.

  If your current locale converts "," to ".", you will need to set
  a locale that doesn't do this (e.g. the "C" locale) before using
  QPSPrinter.

  The TrueType font embedding for Qt's postscript driver uses code
  by David Chappell of Trinity College Computing Center.

  \legalese
  \code

  Copyright 1995, Trinity College Computing Center.
  Written by David Chappell.

  Permission to use, copy, modify, and distribute this software and
  its documentation for any purpose and without fee is hereby
  granted, provided that the above copyright notice appear in all
  copies and that both that copyright notice and this permission
  notice appear in supporting documentation. This software is
  provided "as is" without express or implied warranty.

  TrueType font support. These functions allow PPR to generate
  PostScript fonts from Microsoft compatible TrueType font files.

  The functions in this file do most of the work to convert a
  TrueType font to a type 3 PostScript font.

  Most of the material in this file is derived from a program called
  "ttf2ps" which L. S. Ng posted to the usenet news group
  "comp.sources.postscript". The author did not provide a copyright
  notice or indicate any restrictions on use.

  Last revised 11 July 1995.
  \endcode
*/

/*!
    \enum QPSPrinter::PrinterState

    \value Idle
    \value Active
    \value Aborted
    \value Error
*/

/*!
    \enum QPSPrinter::PrinterMode

    This enum describes the mode the printer should work in. It
    basically presets a certain resolution and working mode.

    \value ScreenResolution Sets the resolution of the print device to
    the screen resolution. This has the big advantage that the results
    obtained when painting on the printer will match more or less
    exactly the visible output on the screen. It is the easiest to
    use, as font metrics on the screen and on the printer are the
    same. This is the default value. ScreenResolution will produce a
    lower quality output than HighResolution and should only be used
    for drafts.

    \value PrinterResolution This value is deprecated. Is is
    equivalent to ScreenResolution on Unix and HighResolution on
    Windows and Mac. Due do the difference between ScreenResolution
    and HighResolution, use of this value may lead to non-portable
    printer code.

    \value HighResolution Use printer resolution on Windows, and set
    the resolution of the Postscript driver to 600dpi.
*/

/*!
  \enum QPSPrinter::Orientation

  This enum type (not to be confused with \c Orientation) is used
  to specify each page's orientation.

  \value Portrait the page's height is greater than its width.

  \value Landscape the page's width is greater than its height.

  This type interacts with \l QPSPrinter::PageSize and
  QPSPrinter::setFullPage() to determine the final size of the page
  available to the application.
*/


/*!
    \enum QPSPrinter::PrintRange
    This enum is here for compatibility

    \value AllPages
    \value Selection
    \value PageRange
*/

/*!
    \enum QPSPrinter::PrinterOption
    This enum is here for compatibility

    \value PrintToFile
    \value PrintSelection
    \value PrintPageRange
*/

/*!
  \enum QPSPrinter::PageSize

  This enum type specifies what paper size QPSPrinter should use.
  QPSPrinter does not check that the paper size is available; it just
  uses this information, together with QPSPrinter::Orientation and
  QPSPrinter::setFullPage(), to determine the printable area.

  The defined sizes (with setFullPage(true)) are:

  \value A0 841 x 1189 mm
  \value A1 594 x 841 mm
  \value A2 420 x 594 mm
  \value A3 297 x 420 mm
  \value A4 210 x 297 mm, 8.26 x 11.69 inches
  \value A5 148 x 210 mm
  \value A6 105 x 148 mm
  \value A7 74 x 105 mm
  \value A8 52 x 74 mm
  \value A9 37 x 52 mm
  \value B0 1030 x 1456 mm
  \value B1 728 x 1030 mm
  \value B10 32 x 45 mm
  \value B2 515 x 728 mm
  \value B3 364 x 515 mm
  \value B4 257 x 364 mm
  \value B5 182 x 257 mm, 7.17 x 10.13 inches
  \value B6 128 x 182 mm
  \value B7 91 x 128 mm
  \value B8 64 x 91 mm
  \value B9 45 x 64 mm
  \value C5E 163 x 229 mm
  \value Comm10E 105 x 241 mm, US Common 10 Envelope
  \value DLE 110 x 220 mm
  \value Executive 7.5 x 10 inches, 191 x 254 mm
  \value Folio 210 x 330 mm
  \value Ledger 432 x 279 mm
  \value Legal 8.5 x 14 inches, 216 x 356 mm
  \value Letter 8.5 x 11 inches, 216 x 279 mm
  \value Tabloid 279 x 432 mm
  \value Custom
  \omitvalue NPageSize

  With setFullPage(false) (the default), the metrics will be a bit
  smaller; how much depends on the printer in use.
*/


/*!
  \enum QPSPrinter::PageOrder

  This enum type is used by QPSPrinter to tell the application program
  how to print.

  \value FirstPageFirst  the lowest-numbered page should be printed
  first.

  \value LastPageFirst  the highest-numbered page should be printed
  first.
*/

/*!
  \enum QPSPrinter::ColorMode

  This enum type is used to indicate whether QPSPrinter should print
  in color or not.

  \value Color  print in color if available, otherwise in grayscale.

  \value GrayScale  print in grayscale, even on color printers.
*/

/*!
  \enum QPSPrinter::PaperSource

  This enum type specifies what paper source QPSPrinter is to use.
  QPSPrinter does not check that the paper source is available; it
  just uses this information to try and set the paper source.
  Whether it will set the paper source depends on whether the
  printer has that particular source.

  \warning This is currently only implemented for Windows.

  \value OnlyOne
  \value Lower
  \value Middle
  \value Manual
  \value Envelope
  \value EnvelopeManual
  \value Auto
  \value Tractor
  \value SmallFormat
  \value LargeFormat
  \value LargeCapacity
  \value Cassette
  \value FormSource
*/

/*
  \enum QPSPrinter::PrintRange

  This enum is used to specify which print range the application
  should use to print.

  \value AllPages All the pages should be printed.
  \value Selection Only the selection should be printed.
  \value PageRange Print according to the from page and to page options.

  \sa setPrintRange(), printRange()
*/

/*
  \enum QPSPrinter::PrinterOption

  This enum describes various printer options that appear in the
  printer setup dialog. It is used to enable and disable these
  options in the setup dialog.

  \value PrintToFile Describes if print to file should be enabled.
  \value PrintSelection Describes if printing selections should be enabled.
  \value PrintPageRange Describes if printing page ranges (from, to) should
  be enabled

  \sa setOptionEnabled(), isOptionEnabled()
*/

/*!
    Creates a new printer object with the given \a mode.
*/
QPSPrinter::QPSPrinter(PrinterMode mode)
    : QPaintDevice(),
      d_ptr(new QPSPrinterPrivate)
{
    Q_D(QPSPrinter);
    d->printEngine = new QPSPrintEngine(mode);
}

/*!
    Destroys the printer object and frees any allocated resources. If
    the printer is destroyed while a print job is in progress this may
    or may not affect the print job.
*/
QPSPrinter::~QPSPrinter()
{
    Q_D(QPSPrinter);
#ifdef QT3_SUPPORT
    delete d->printDialog;
#endif
    delete d->printEngine;
    delete d;
}

/*! \reimp */
int QPSPrinter::devType() const
{
    return QInternal::Printer;
}

/*!
    Returns the printer name. This value is initially set to the name
    of the default printer.

    \sa setPrinterName()
*/
QString QPSPrinter::printerName() const
{
    Q_D(const QPSPrinter);
    return d->printEngine->property(QPrintEngine::PPK_PrinterName).toString();
}

/*!
    Sets the printer name to \a name.

    \sa printerName()
*/
void QPSPrinter::setPrinterName(const QString &name)
{
    Q_D(QPSPrinter);
    ABORT_IF_ACTIVE("QPSPrinter::setPrinterName()");
    d->printEngine->setProperty(QPrintEngine::PPK_PrinterName, name);
}


/*!
  \fn bool QPSPrinter::outputToFile() const

  Returns true if the output should be written to a file, or false
  if the output should be sent directly to the printer. The default
  setting is false.

  \sa setOutputToFile(), setOutputFileName()
*/


/*!
  \fn void QPSPrinter::setOutputToFile(bool enable)

  Specifies whether the output should be written to a file or sent
  directly to the printer.

  Will output to a file if \a enable is true, or will output
  directly to the printer if \a enable is false.

  \sa outputToFile(), setOutputFileName()
*/


/*!
  \fn QString QPSPrinter::outputFileName() const

  Returns the name of the output file. By default, this is an empty string
  (indicating that the printer shouldn't print to file).

  \sa setOutputFileName()
*/

QString QPSPrinter::outputFileName() const
{
    Q_D(const QPSPrinter);
    return d->printEngine->property(QPrintEngine::PPK_OutputFileName).toString();
}

/*!
  Sets the name of the output file to \a fileName.

  Setting a null or empty name (0 or "") disables printing to a file. Setting a
  non-empty name enables printing to a file.

  \sa outputFileName(), setOutputToFile()
*/

void QPSPrinter::setOutputFileName(const QString &fileName)
{
    Q_D(QPSPrinter);
    ABORT_IF_ACTIVE("QPSPrinter::setOutputFileName()");
    d->printEngine->setProperty(QPrintEngine::PPK_OutputFileName, fileName);
}


/*!
  Returns the name of the program that sends the print output to the
  printer.

  The default is to return an empty string; meaning that QPSPrinter will try to
  be smart in a system-dependent way. On X11 only, you can set it to something
  different to use a specific print program. On the other platforms, this
  returns an empty string.

  \sa setPrintProgram(), setPrinterSelectionOption()
*/
QString QPSPrinter::printProgram() const
{
    Q_D(const QPSPrinter);
    return d->printEngine->property(QPrintEngine::PPK_PrinterProgram).toString();
}


/*!
  Sets the name of the program that should do the print job to \a
  printProg.

  On X11, this function sets the program to call with the PostScript
  output. On other platforms, it has no effect.

  \sa printProgram()
*/
void QPSPrinter::setPrintProgram(const QString &printProg)
{
    Q_D(QPSPrinter);
    ABORT_IF_ACTIVE("QPSPrinter::setPrintProgram()");
    d->printEngine->setProperty(QPrintEngine::PPK_PrinterProgram, printProg);
}


/*!
  Returns the document name.

  \sa setDocName()
*/
QString QPSPrinter::docName() const
{
    Q_D(const QPSPrinter);
    return d->printEngine->property(QPrintEngine::PPK_DocumentName).toString();
}


/*!
  Sets the document name to \a name.
*/
void QPSPrinter::setDocName(const QString &name)
{
    Q_D(QPSPrinter);
    ABORT_IF_ACTIVE("QPSPrinter::setDocName()");
    d->printEngine->setProperty(QPrintEngine::PPK_DocumentName, name);
}


/*!
  Returns the name of the application that created the document.

  \sa setCreator()
*/
QString QPSPrinter::creator() const
{
    Q_D(const QPSPrinter);
    return d->printEngine->property(QPrintEngine::PPK_Creator).toString();
}


/*!
  Sets the name of the application that created the document to \a
  creator.

  This function is only applicable to the X11 version of Qt. If no
  creator name is specified, the creator will be set to "Qt"
  followed by some version number.

  \sa creator()
*/
void QPSPrinter::setCreator(const QString &creator)
{
    Q_D(QPSPrinter);
    ABORT_IF_ACTIVE("QPSPrinter::setCreator");
    d->printEngine->setProperty(QPrintEngine::PPK_Creator, creator);
}


/*!
  Returns the orientation setting. This is driver-dependent, but is usually
  QPSPrinter::Portrait.

  \sa setOrientation()
*/
QPSPrinter::Orientation QPSPrinter::orientation() const
{
    Q_D(const QPSPrinter);
    return QPSPrinter::Orientation(d->printEngine->property(QPrintEngine::PPK_Orientation).toInt());
}


/*!
  Sets the print orientation to \a orientation.

  The orientation can be either QPSPrinter::Portrait or
  QPSPrinter::Landscape.

  The printer driver reads this setting and prints using the
  specified orientation.

  On Windows and Mac OS X, this option can be changed while printing and will
  take effect from the next call to newPage().

  \sa orientation()
*/

void QPSPrinter::setOrientation(Orientation orientation)
{
    Q_D(QPSPrinter);
    d->printEngine->setProperty(QPrintEngine::PPK_Orientation, orientation);
}


/*!
  Returns the printer page size. The default value is driver-dependent.

  \sa setPageSize() pageRect() paperRect()
*/
QPSPrinter::PageSize QPSPrinter::pageSize() const
{
    Q_D(const QPSPrinter);
    return QPSPrinter::PageSize(d->printEngine->property(QPrintEngine::PPK_PageSize).toInt());
}


/*!
  Sets the printer page size to \a newPageSize if that size is
  supported. The result if undefined if \a newPageSize is not
  supported.

  The default page size is driver-dependent.

  This function is useful mostly for setting a default value that
  the user can override in the print dialog.

  \sa pageSize() PageSize setFullPage() setResolution() pageRect() paperRect()
*/

void QPSPrinter::setPageSize(PageSize newPageSize)
{
    Q_D(QPSPrinter);
    ABORT_IF_ACTIVE("QPSPrinter::setPageSize()");
    if (newPageSize > NPageSize) {
        qWarning("QPSPrinter::SetPageSize: illegal page size %d", newPageSize);
        return;
    }
    d->printEngine->setProperty(QPrintEngine::PPK_PageSize, newPageSize);
}

/*!
    Sets the page order to \a pageOrder.

    The page order can be \c QPSPrinter::FirstPageFirst or \c
    QPSPrinter::LastPageFirst. The application is responsible for
    reading the page order and printing accordingly.

    This function is mostly useful for setting a default value that
    the user can override in the print dialog.
*/

void QPSPrinter::setPageOrder(PageOrder pageOrder)
{
    Q_D(QPSPrinter);
    ABORT_IF_ACTIVE("QPSPrinter::setPageOrder()");
    d->printEngine->setProperty(QPrintEngine::PPK_PageOrder, pageOrder);
}


/*!
  Returns the current page order.

  The default page order is \c FirstPageFirst.
*/

QPSPrinter::PageOrder QPSPrinter::pageOrder() const
{
    Q_D(const QPSPrinter);
    return QPSPrinter::PageOrder(d->printEngine->property(QPrintEngine::PPK_PageOrder).toInt());
}


/*!
  Sets the printer's color mode to \a newColorMode, which can be
  either \c Color or \c GrayScale.

  \sa colorMode()
*/

void QPSPrinter::setColorMode(ColorMode newColorMode)
{
    Q_D(QPSPrinter);
    ABORT_IF_ACTIVE("QPSPrinter::setColorMode()");
    d->printEngine->setProperty(QPrintEngine::PPK_ColorMode, newColorMode);
}


/*!
  Returns the current color mode.

  \sa setColorMode()
*/
QPSPrinter::ColorMode QPSPrinter::colorMode() const
{
    Q_D(const QPSPrinter);
    return QPSPrinter::ColorMode(d->printEngine->property(QPrintEngine::PPK_ColorMode).toInt());
}


/*!
  Returns the number of copies to be printed. The default value is 1.

  On Windows and Mac OS X, this will always return 1 as these operating systems
  can internally handle the number of copies.

  On X11, this value will return the number of times the application is
  required to print in order to match the number specified in the printer setup
  dialog. This has been done since some printer drivers are not capable of
  buffering up the copies and in those cases the application must make an
  explicit call to the print code for each copy.

  \sa setNumCopies()
*/

int QPSPrinter::numCopies() const
{
    Q_D(const QPSPrinter);
   return d->printEngine->property(QPrintEngine::PPK_NumberOfCopies).toInt();
}


/*!
  Sets the number of copies to be printed to \a numCopies.

  The printer driver reads this setting and prints the specified
  number of copies.

  \sa numCopies()
*/

void QPSPrinter::setNumCopies(int numCopies)
{
    Q_D(QPSPrinter);
    ABORT_IF_ACTIVE("QPSPrinter::setNumCopies()");
    d->printEngine->setProperty(QPrintEngine::PPK_NumberOfCopies, numCopies);
}


/*!
  \internal

  Returns true if collation is turned on when multiple copies is selected.
  Returns false if it is turned off when multiple copies is selected.

  \sa collateCopiesEnabled() setCollateCopiesEnabled() setCollateCopies()
*/
bool QPSPrinter::collateCopies() const
{
    Q_D(const QPSPrinter);
    return d->printEngine->property(QPrintEngine::PPK_CollateCopies).toBool();
}


/*!
  \internal

  Sets the default value for collation checkbox when the print dialog appears.
  If \a on is true, it will enable setCollateCopiesEnabled().
  The default value is false. This value will be changed by what the
  user presses in the print dialog.

  \sa collateCopiesEnabled() setCollateCopiesEnabled() collateCopies()
*/
void QPSPrinter::setCollateCopies(bool collate)
{
    Q_D(QPSPrinter);
    ABORT_IF_ACTIVE("QPSPrinter::setCollateCopies()");
    d->printEngine->setProperty(QPrintEngine::PPK_CollateCopies, collate);
}



/*!
  Sets QPSPrinter to have the origin of the coordinate system at the
  top-left corner of the paper if \a fp is true, or where it thinks
  the top-left corner of the printable area is if \a fp is false.

  The default is false. You can (probably) print on (0,0), and
  the device metrics will report something smaller than the size
  indicated by PageSize. (Note that QPSPrinter may be wrong on Unix
  systems: it does not have perfect knowledge of the physical
  printer.)

  If \a fp is true, the device metrics will report the exact same
  size as indicated by \c PageSize. It probably isn't possible to
  print on the entire page because of the printer's physical
  margins, so the application must account for the margins itself.

  \sa PageSize setPageSize() fullPage() width() height()
*/

void QPSPrinter::setFullPage(bool fp)
{
    Q_D(QPSPrinter);
    d->printEngine->setProperty(QPrintEngine::PPK_FullPage, fp);
}


/*!
  Returns true if the origin of the printer's coordinate system is
  at the corner of the page and false if it is at the edge of the
  printable area.

  See setFullPage() for details and caveats.

  \sa setFullPage() PageSize
*/

bool QPSPrinter::fullPage() const
{
    Q_D(const QPSPrinter);
    return d->printEngine->property(QPrintEngine::PPK_FullPage).toBool();
}


/*!
  Requests that the printer prints at \a dpi or as near to \a dpi as
  possible.

  This setting affects the coordinate system as returned by, for
  example QPainter::viewport().

  This function must be called before QPainter::begin() to have an effect on
  all platforms.

  \sa resolution() setPageSize()
*/

void QPSPrinter::setResolution(int dpi)
{
    Q_D(QPSPrinter);
    ABORT_IF_ACTIVE("QPSPrinter::setResolution()");
    d->printEngine->setProperty(QPrintEngine::PPK_Resolution, dpi);
}


/*!
  Returns the current assumed resolution of the printer, as set by
  setResolution() or by the printer driver.

  \sa setResolution()
*/

int QPSPrinter::resolution() const
{
    Q_D(const QPSPrinter);
    return d->printEngine->property(QPrintEngine::PPK_Resolution).toInt();
}

/*!
  Sets the paper source setting to \a source.

  Windows only: This option can be changed while printing and will
  take effect from the next call to newPage()

  \sa paperSource()
*/

void QPSPrinter::setPaperSource(PaperSource source)
{
    Q_D(QPSPrinter);
    d->printEngine->setProperty(QPrintEngine::PPK_PaperSource, source);
}

/*!
    Returns the printer's paper source. This is \c Manual or a printer
    tray or paper cassette.
*/
QPSPrinter::PaperSource QPSPrinter::paperSource() const
{
    Q_D(const QPSPrinter);
    return QPSPrinter::PaperSource(d->printEngine->property(QPrintEngine::PPK_PaperSource).toInt());
}

/*!
    Returns the page's rectangle; this is usually smaller than the
    paperRect() since the page normally has margins between its
    borders and the paper.

    \sa pageSize()
*/
QRect QPSPrinter::pageRect() const
{
    Q_D(const QPSPrinter);
    return d->printEngine->property(QPrintEngine::PPK_PageRect).toRect();
}

/*!
    Returns the paper's rectangle; this is usually larger than the
    pageRect().

   \sa pageRect()
*/
QRect QPSPrinter::paperRect() const
{
    Q_D(const QPSPrinter);
    return d->printEngine->property(QPrintEngine::PPK_PaperRect).toRect();
}

/*!
    \internal

    Returns the metric for the given \a id.
*/
int QPSPrinter::metric(PaintDeviceMetric id) const
{
    Q_D(const QPSPrinter);
    return d->printEngine->metric(id);
}

/*!
    Returns the paint engine used by the printer.
*/
QPaintEngine *QPSPrinter::paintEngine() const

{
    Q_D(const QPSPrinter);
// Being a bit safe, since we have multiple inheritance...
    return static_cast<QPSPrintEngine *>(d->printEngine);
}


#if defined (Q_WS_WIN)
/*!
    Sets the page size to be used by the printer under Windows to \a
    pageSize.

    \warning This function is not portable so you may prefer to use
    setPageSize() instead.

    \sa winPageSize()
*/
void QPSPrinter::setWinPageSize(int pageSize)
{
    Q_D(QPSPrinter);
    ABORT_IF_ACTIVE("QPSPrinter::setWinPageSize()");
    d->printEngine->setProperty(QPrintEngine::PPK_WindowsPageSize, pageSize);
}

/*!
    Returns the page size used by the printer under Windows.

    \warning This function is not portable so you may prefer to use
    pageSize() instead.

    \sa setWinPageSize()
*/
int QPSPrinter::winPageSize() const
{
    Q_D(const QPSPrinter);
    return d->printEngine->property(QPrintEngine::PPK_WindowsPageSize).toInt();
}
#endif // Q_WS_WIN

/*!
    Returns a list of the resolutions (a list of dots-per-inch
    integers) that the printer says it supports.

    For X11 where all printing is directly to postscript, this
    function will always return a one item list containing only the
    postscript resolution, i.e., 72 (72 dpi -- but see PrinterMode).
*/
QList<int> QPSPrinter::supportedResolutions() const
{
    Q_D(const QPSPrinter);
    QList<QVariant> varlist
        = d->printEngine->property(QPrintEngine::PPK_SupportedResolutions).toList();
    QList<int> intlist;
    for (int i=0; i<varlist.size(); ++i)
        intlist << varlist.at(i).toInt();
    return intlist;
}

/*!
    Tells the printer to eject the current page and to continue
    printing on a new page. Returns true if this was successful;
    otherwise returns false.
*/
bool QPSPrinter::newPage()
{
    Q_D(QPSPrinter);
    return d->printEngine->newPage();
}

/*!
    Aborts the current print run. Returns true if the print run was
    successfully aborted and printerState() will return QPSPrinter::Aborted; otherwise
    returns false.

    It is not always possible to abort a print job. For example,
    all the data has gone to the printer but the printer cannot or
    will not cancel the job when asked to.
*/
bool QPSPrinter::abort()
{
    Q_D(QPSPrinter);
    return d->printEngine->abort();
}

#endif // QT_NO_PRINTER

#if 0
/*!
  \fn int QPSPrinter::minPage() const

  Returns the min-page setting, i.e. the lowest page number a user
  is allowed to choose. The default value is 0 which signifies 'any
  page'.

  \sa maxPage(), setMinMax() setFromTo()
*/

/*!
  \fn int QPSPrinter::maxPage() const

  Returns the max-page setting. A user can't choose a higher page
  number than maxPage() when they select a print range. The default
  value is 0 which signifies the last page (up to a maximum of
  9999).

  \sa minPage(), setMinMax() setFromTo()
*/

/*!
  Sets the min-page and max-page settings to \a minPage and \a
  maxPage respectively.

  The min-page and max-page restrict the from-page and to-page
  settings. When the printer setup dialog appears, the user cannot
  select a from page or a to page that are outside the range
  specified by min and max pages.

  \sa minPage(), maxPage(), setFromTo(), setup()
*/

void QPSPrinter::setMinMax(int minPage, int maxPage)
{
    min_pg = minPage;
    max_pg = maxPage;
    if (from_pg == 0 || from_pg < minPage)
	from_pg = minPage;
    if (to_pg == 0 || to_pg > maxPage)
	to_pg = maxPage;
}

/*!
  \fn bool QPSPrinter::collateCopiesEnabled() const

  \internal

  Returns true if the application should provide the user with the
  option of choosing a collated printout; otherwise returns false.

  Collation means that each page is printed in order, i.e. print the
  first page, then the second page, then the third page and so on, and
  then repeat this sequence for as many copies as have been requested.
  If you don't collate you get several copies of the first page, then
  several copies of the second page, then several copies of the third
  page, and so on.

  \sa setCollateCopiesEnabled() setCollateCopies() collateCopies()
*/

/*!
  \fn void QPSPrinter::setCollateCopiesEnabled(bool enable)

  \internal

  If \a enable is true (the default) the user is given the choice of
  whether to print out multiple copies collated in the print dialog.
  If \a enable is false, then collateCopies() will be ignored.

  Collation means that each page is printed in order, i.e. print the
  first page, then the second page, then the third page and so on, and
  then repeat this sequence for as many copies as have been requested.
  If you don't collate you get several copies of the first page, then
  several copies of the second page, then several copies of the third
  page, and so on.

  \sa collateCopiesEnabled() setCollateCopies() collateCopies()
*/


#endif

/*!
    Returns the current state of the printer. This may not always be
    accurate (for example if the printer doesn't have the capability
    of reporting its state to the operating system).
*/
 QPrinter::PrinterState QPSPrinter::printerState() const
{
     Q_D(const QPSPrinter);
     return (QPrinter::PrinterState) d->printEngine->printerState();
 }


/*! \fn void QPSPrinter::margins(uint *top, uint *left, uint *bottom, uint *right) const

    Sets *\a top, *\a left, *\a bottom, *\a right to be the top,
    left, bottom, and right margins.

    This function has been superceded by paperRect() and pageRect().
    Use pageRect().top() - paperRect().top() for the top margin,
    pageRect().left() - paperRect().left() for the left margin,
    pageRect().bottom() - paperRect().bottom() for the bottom margin,
    and pageRect().right() - paperRect().right() for the right
    margin.

    \oldcode
        uint rightMargin;
        uint bottomMargin;
        printer->margins(0, 0, &bottomMargin, &rightMargin);
    \newcode
        int rightMargin = printer->pageRect().right() - printer->paperRect().right();
        int bottomMargin = printer->pageRect().bottom() - printer->paperRect().bottom();
    \endcode
*/

/*! \fn QSize QPSPrinter::margins() const

    \overload

    Returns a QSize containing the left margin and the top margin.

    This function has been superceded by paperRect() and pageRect().
    Use pageRect().left() - paperRect().left() for the left margin,
    and pageRect().top() - paperRect().top() for the top margin.

    \oldcode
        QSize margins = printer->margins();
        int leftMargin = margins.width();
        int topMargin = margins.height();
    \newcode
        int leftMargin = printer->pageRect().left() - printer->paperRect().left();
        int topMargin = printer->pageRect().top() - printer->paperRect().top();
    \endcode
*/

/*! \fn bool QPSPrinter::aborted()

    Use printerState() == QPSPrinter::Aborted instead.
*/

#ifdef Q_WS_WIN
/*!
    \internal
*/
HDC QPSPrinter::getDC() const
{
    Q_D(const QPSPrinter);
    return d->printEngine->getPrinterDC();
}

/*!
    \internal
*/
void QPSPrinter::releaseDC(HDC hdc) const
{
    Q_D(const QPSPrinter);
    d->printEngine->releasePrinterDC(hdc);
}
#endif

/*!
    \fn QString QPSPrinter::printerSelectionOption() const

    Returns the printer options selection string. This is useful only
    if the print command has been explicitly set.

    The default value (an empty string) implies that the printer should
    be selected in a system-dependent manner.

    Any other value implies that the given value should be used.

    \warning This function is not available on Windows.

    \sa setPrinterSelectionOption()
*/

/*!
    \fn void QPSPrinter::setPrinterSelectionOption(const QString &option)

    Sets the printer to use \a option to select the printer. \a option
    is null by default (which implies that Qt should be smart enough
    to guess correctly), but it can be set to other values to use a
    specific printer selection option.

    If the printer selection option is changed while the printer is
    active, the current print job may or may not be affected.

    \warning This function is not available on Windows.

    \sa printerSelectionOption()
*/

#ifndef Q_WS_WIN
QString QPSPrinter::printerSelectionOption() const
{
    Q_D(const QPSPrinter);
    return d->printEngine->property(QPrintEngine::PPK_SelectionOption).toString();
}

void QPSPrinter::setPrinterSelectionOption(const QString &option)
{
    Q_D(QPSPrinter);
    d->printEngine->setProperty(QPrintEngine::PPK_SelectionOption, option);
}
#endif

#ifdef QT3_SUPPORT

void QPSPrinter::setOutputToFile(bool f)
{
    if (f) {
        if (outputFileName().isEmpty())
            setOutputFileName("untitled_printer_document");
    } else {
        setOutputFileName(QString());
    }
}

bool qt_compat_QPSPrinter_printSetup(QPSPrinter *p, QPSPrinterPrivate *pd, QWidget *parent)
{
  return true;
}


#ifdef Q_WS_MAC
bool qt_compat_QPSPrinter_pageSetup(QPSPrinter *p, QWidget *parent)
{
    QPageSetupDialog psd(p, parent);
    return psd.exec() != 0;
}

/*!
    Executes a page setup dialog so that the user can configure the type of
    page used for printing. Returns true if the contents of the dialog are
    accepted; returns false if the dialog is canceled.
*/
bool QPSPrinter::pageSetup(QWidget *parent)
{
    return qt_compat_QPSPrinter_pageSetup(this, parent);
}

/*!
    Executes a print setup dialog so that the user can configure the printing
    process. Returns true if the contents of the dialog are accepted; returns
    false if the dialog is canceled.
*/
bool QPSPrinter::printSetup(QWidget *parent)
{
    Q_D(QPSPrinter);
    return qt_compat_QPSPrinter_printSetup(this, d, parent);
}
#endif // Q_WS_MAC

/*!
    \compat

    Use QPrintDialog instead.

    \oldcode
        if (printer->setup(parent))
            ...
    \newcode
        QPrintDialog dialog(printer, parent);
        if (dialog.exec())
            ...
    \endcode
*/
bool QPSPrinter::setup(QWidget *parent)
{
    Q_D(QPSPrinter);
    return qt_compat_QPSPrinter_printSetup(this, d, parent)
#ifdef Q_WS_MAC
        && qt_compat_QPSPrinter_pageSetup(this, parent);
#endif
        ;
}

/*!
    \fn int QPSPrinter::fromPage() const

    \compat

    Use QPrintDialog instead.

    Returns the from-page setting. The default value is 0.

    If fromPage() and toPage() both return 0 this signifies 'print the
    whole document'.

    The programmer is responsible for reading this setting and
    printing accordingly.


    \sa setFromTo(), toPage()
*/

int QPSPrinter::fromPage() const
{
}

/*!
    \fn int QPSPrinter::toPage() const

    \compat

    Use QPrintDialog instead.

    Returns the to-page setting. The default value is 0.

    If fromPage() and toPage() both return 0 this signifies 'print the
    whole document'.

    The programmer is responsible for reading this setting and
    printing accordingly.

    \sa setFromTo(), fromPage()
*/

int QPSPrinter::toPage() const
{
}

/*!
    \compat

    Use QPrintDialog instead.

    Sets the from-page and to-page settings to \a from and \a
    to respectively.

    The from-page and to-page settings specify what pages to print.

    If from and to both return 0 this signifies 'print the whole
    document'.

    This function is useful mostly to set a default value that the
    user can override in the print dialog when you call setup().

    Use QPrintDialog instead.

    \sa fromPage(), toPage(), setMinMax(), setup()
*/

void QPSPrinter::setFromTo(int from, int to)
{
}

/*!
    \fn int QPSPrinter::minPage() const

    \compat

    Use QPrintDialog instead.

    Returns the min-page setting, i.e. the lowest page number a user
    is allowed to choose. The default value is 0.

    \sa maxPage(), setMinMax() setFromTo()
*/
int QPSPrinter::minPage() const
{
}

/*!
    \fn int QPSPrinter::maxPage() const

    \compat

    Use QPrintDialog instead.

    Returns the max-page setting. A user can't choose a higher page
    number than maxPage() when they select a print range. The default
    value is 0.

    \sa minPage(), setMinMax() setFromTo()
*/

int QPSPrinter::maxPage() const
{
}

/*!
    \compat

    Use QPrintDialog instead.

    Sets the min-page and max-page settings to \a minPage and \a
    maxPage respectively.

    The min-page and max-page restrict the from-page and to-page
    settings. When the printer setup dialog appears, the user cannot
    select a from page or a to page that are outside the range
    specified by min and max pages.

    \sa minPage(), maxPage(), setFromTo(), setup()
*/

void QPSPrinter::setMinMax( int minPage, int maxPage )
{
}

/*!
  \fn bool QPSPrinter::collateCopiesEnabled() const

  \compat

  Use QPrintDialog instead

  \internal

  Returns true if the application should provide the user with the
  option of choosing a collated printout; otherwise returns false.

  Collation means that each page is printed in order, i.e. print the
  first page, then the second page, then the third page and so on, and
  then repeat this sequence for as many copies as have been requested.
  If you don't collate you get several copies of the first page, then
  several copies of the second page, then several copies of the third
  page, and so on.

  \sa setCollateCopiesEnabled() setCollateCopies() collateCopies()
*/

bool QPSPrinter::collateCopiesEnabled() const
{
}

/*!
    \fn void QPSPrinter::setCollateCopiesEnabled(bool enable)

    \compat

    Use QPrintDialog instead.

    \internal

    If \a enable is true (the default) the user is given the choice of
    whether to print out multiple copies collated in the print dialog.
    If \a enable is false, then collateCopies() will be ignored.

    Collation means that each page is printed in order, i.e. print the
    first page, then the second page, then the third page and so on, and
    then repeat this sequence for as many copies as have been requested.
    If you don't collate you get several copies of the first page, then
    several copies of the second page, then several copies of the third
    page, and so on.

  \sa collateCopiesEnabled() setCollateCopies() collateCopies()
*/

void QPSPrinter::setCollateCopiesEnabled(bool enable)
{
}

/*!
    \compat

    Use QPrintDialog instead.

    Sets the default selected page range to be used when the print setup
    dialog is opened to \a range. If the PageRange specified by \a range is
    currently disabled the function does nothing.

    \sa printRange()
*/

void QPSPrinter::setPrintRange( PrintRange range )
{
}

/*!
    \compat

    Use QPrintDialog instead.

    Returns the PageRange of the QPSPrinter. After the print setup dialog
    has been opened, this function returns the value selected by the user.

    \sa setPrintRange()
*/
QPSPrinter::PrintRange QPSPrinter::printRange() const
{
}

/*!
    \compat

    Use QPrintDialog instead.

    Enables the printer option with the identifier \a option if \a
    enable is true, and disables option \a option if \a enable is false.

    \sa isOptionEnabled()
*/
void QPSPrinter::setOptionEnabled( PrinterOption option, bool enable )
{
}

/*!
    \compat

    Use QPrintDialog instead.

  Returns true if the printer option with identifier \a option is enabled;
  otherwise returns false.

  \sa setOptionEnabled()
 */
bool QPSPrinter::isOptionEnabled( PrinterOption option ) const
{
  return false;
}

#endif // QT3_SUPPORT

/*!
    \class QPrintEngine

    \brief The QPrintEngine class defines an interface for how QPSPrinter
    interacts with a given printing subsystem.

    The common case when creating your own print engine is to derive from both
    QPaintEngine and QPrintEngine. Various properties of a print engine are
    given with property() and set with setProperty().

    \sa QPaintEngine
*/

/*!
    \enum QPrintEngine::PrintEnginePropertyKey

    This enum is used to communicate properties between the print
    engine and QPSPrinter. A property may or may not be supported by a
    given print engine.

    \value PPK_CollateCopies A bool value describing wether the
    printout should be collated or not.

    \value PPK_ColorMode Refers to QPSPrinter::ColorMode, either color or
    monochrome.

    \value PPK_Creator

    \value PPK_DocumentName A string describing the document name in
    the spooler.

    \value PPK_FullPage A boolean describing if the printer should be
    full page or not.

    \value PPK_NumberOfCopies An integer specifying the number of
    copies

    \value PPK_Orientation Specifies a QPSPrinter::Orientation value.

    \value PPK_OutputFileName The output file name as a string. An
    empty file name indicates that the printer should not print to a file.

    \value PPK_PageOrder Specifies a QPSPrinter::PageOrder value.

    \value PPK_PageRect A QRect specifying the page rectangle

    \value PPK_PageSize Specifies a QPSPrinter::PageSize value.

    \value PPK_PaperRect A QRect specifying the paper rectangle.

    \value PPK_PaperSource Specifies a QPSPrinter::PaperSource value.

    \value PPK_PrinterName A string specifying the name of the printer.

    \value PPK_PrinterProgram A string specifying the name of the
    printer program used for printing,

    \value PPK_Resolution An integer describing the dots per inch for
    this printer.

    \value PPK_SelectionOption

    \value PPK_SupportedResolutions A list of integer QVariants
    describing the set of supported resolutions that the printer has.

    \value PPK_WindowsPageSize An integer specifying a DM_PAPER entry
    on Windows.

    \value PPK_CustomBase Basis for extension.
*/

/*!
    \fn QPrintEngine::~QPrintEngine()

    Destroys the print engine.
*/

/*!
    \fn void QPrintEngine::setProperty(PrintEnginePropertyKey key, const QVariant &value)

    Sets the print engine's property specified by \a key to the given \a value.

    \sa property()
*/

/*!
    \fn void QPrintEngine::property(PrintEnginePropertyKey key) const

    Returns the print engine's property specified by \a key.

    \sa setProperty()
*/

/*!
    \fn bool QPrintEngine::newPage()

    Instructs the print engine to start a new page. Returns true if
    the printer was able to create the new page; otherwise returns false.
*/

/*!
    \fn bool QPrintEngine::abort()

    Instructs the print engine to abort the printing process. Returns
    true if successful; otherwise returns false.
*/

/*!
    \fn int QPrintEngine::metric(QPaintDevice::PaintDeviceMetric id) const

    Returns the metric for the given \a id.
*/

/*!
    \fn QPSPrinter::PrinterState QPrintEngine::printerState() const

    Returns the current state of the printer being used by the print engine.
*/

/*!
    \fn HDC QPrintEngine::getPrinterDC() const
    \internal
*/

/*!
    \fn void QPrintEngine::releasePrinterDC(HDC) const
    \internal
*/
