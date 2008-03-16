/*
 * Copyright (c) 2002-2007 Samit Basu, Eugene Ingerman
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
 *
 */

#include "blas_wrap.h"
#include "blas_dyn_link.h"

#include <iostream>
#include <vector>
#include <QCoreApplication>
#include <QString>
#include <QStringList>
#include <QLibrary> 
#include <QSettings>
#include <QSysInfo>
#include "Exception.hpp"

BlasWrapper::BlasWrapper()
{  
}

void BlasWrapper::Init( void )
{
    blasLib = new QLibrary();
    InitFunctions();
    DiscoverBlasLibrary();
    if( libList.empty() )
	throw Exception("No appropriate BLAS libraries found!");
    LoadLib( *(libList.begin()) );
}

std::string BlasWrapper::ComputerType( void )
{
#ifdef Q_WS_WIN
    if( QSysInfo::WindowsVersion & QSysInfo::WV_NT_based )
	return std::string("Win32");
    throw Exception("Unsupported Windows version.");
#endif
#ifdef Q_WS_WIN
    return std::string( "OSX" );
#endif

    switch( QSysInfo::WordSize ){
	case 32:
	    return std::string( "Linux32" );
	    break;
	case 64:
	    return std::string( "Linux64" );
	    break;
    }
    throw Exception( "Unknown word size." );
}

void BlasWrapper::ListLibraries( std::string& msg )
{
    std::list<LibConf>::iterator it = libList.begin();

    msg.clear();
    while( it != libList.end() ){
	msg.append( it->Name.toStdString() + "\t-\t" + it->desc.toStdString() + "\n" );
	++it;
    }
}

bool BlasWrapper::LoadLibByName( const std::string& name, std::string& msg )
{
    std::list<LibConf>::iterator it = libList.begin();
    while( it != libList.end() ){
	if( it->Name.toStdString() == name ){
	    LoadLib( *it );
	    return true;
	}
	++it;
    }
    return false;
}

void BlasWrapper::LoadLib( const LibConf& libConf )
{
    InitFunctions();  //clear function pointers to reload the library
    blasLib->unload(); //unload the old libarary

    currentLib = libConf;
    blasLib->setFileName( currentLib.fileName );
    if( !blasLib->load() )
	throw Exception( blasLib->errorString().toStdString() );
}

void BlasWrapper::DiscoverBlasLibrary( void )
{
    QSettings *libsettings = new QSettings( QCoreApplication::applicationDirPath()+"/blas.ini", QSettings::IniFormat );
    libsettings->beginGroup( ComputerType().c_str() );

    QStringList libs = libsettings->childGroups();
    QStringListIterator libs_it(libs);

    while( libs_it.hasNext() ){
	QString lib_kind = libs_it.next();
	libsettings->beginGroup( lib_kind );
	
	//we try loading every library we find in config file.
	blasLib->setFileName( libsettings->value( "libname" ).toString() );
	if( blasLib->load() ){ 
	    LibConf lib;
	    lib.Name = lib_kind;
	    lib.fileName = libsettings->value( "libname" ).toString();
	    lib.capitalized = libsettings->value( "capfnames" ).toBool();
	    lib.prefix = libsettings->value( "prefix" ).toString();
	    lib.suffix = libsettings->value( "suffix" ).toString();
	    lib.desc = libsettings->value( "desc" ).toString();
	    libList.push_back( lib );
	    blasLib->unload();
	}
	libsettings->endGroup();
    }
    delete libsettings;
    return;
}

void* BlasWrapper::Resolve( const char* function_name )
{
    void *p;
    QString fname( function_name );
    if( currentLib.capitalized )
	fname = fname.toUpper();
    //TODO: handle prefix, suffix.
    fname = currentLib.prefix + fname + currentLib.suffix;
    if( !(p = blasLib->resolve( fname.toAscii() ))){
	throw Exception( blasLib->errorString().toStdString() );
    }
    return p;
}


void BlasWrapper::InitFunctions( void )
{
    fn.resize( BLAS_NUM_FNS );
    std::vector<void*>::iterator it = fn.begin();
    while( it != fn.end() )
	*(it++) = 0;
}

extern BlasWrapper wrapper;
void LoadBlasFunctions( void )
{
    wrapper.Init();
}