/*
 * Copyright (c) 2002-2006 Samit Basu
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
#ifdef WIN32
#include <windows.h>
#else
#include <sys/types.h>
#include <sys/stat.h>
#include <dirent.h>
#include <glob.h>
#include <unistd.h>
#include <pwd.h>
#endif

#include "Interface.hpp"
#include "Context.hpp"
#include "File.hpp"
#include <algorithm>
#include <QtCore>
#include <qglobal.h>

#ifdef WIN32
#define DELIM "\\"
#define S_ISREG(x) (x & _S_IFREG)
#include <direct.h>
#define PATH_DELIM ";"
#else
#define DELIM "/"
#define PATH_DELIM ":"
#endif

namespace FreeMat {

  char* TildeExpand(char* path) {
#ifdef WIN32
    return path;
#else
    char *newpath = path;
    if (path[0] == '~' && (path[1] == '/') || (path[1] == 0)) {
      char *home;
      home = getenv("HOME");
      if (home) {
	newpath = (char*) malloc(strlen(path) + strlen(home));
	strcpy(newpath,home);
	strcat(newpath,path+1);
      }
    } else if (path[0] == '~' && isalpha(path[1])) {
      char username[4096];
      char *cp, *dp;
      // Extract the user name
      cp = username;
      dp = path+1;
      while (*dp != '/')
	*cp++ = *dp++;
      *cp = 0;
      // Call getpwnam...
      struct passwd *dat = getpwnam(cp);
      if (dat) {
	newpath = (char*) malloc(strlen(path) + strlen(dat->pw_dir));
	strcpy(newpath,dat->pw_dir);
	strcat(newpath,dp);
      }
    }
    return newpath;
#endif
  }
  
  Interface::Interface() : QObject() {
    m_context = NULL;
  }

  Interface::~Interface() {
  }

  void Interface::setContext(Context *ctxt) {
    if (m_context) delete m_context;
    m_context = ctxt;
  }

  Context* Interface::getContext() {
    return m_context;
  }

  void Interface::setAppPath(std::string path) {
    app_path = path;
  }

  std::string Interface::getAppPath() {
    return app_path;
  }

  void Interface::setPath(std::string path) {
    char* pathdata = strdup(path.c_str());
    // Search through the path
    char *saveptr = (char*) malloc(sizeof(char)*1024);
    char* token;
    token = strtok(pathdata,PATH_DELIM);
    m_userPath.clear();
    while (token != NULL) {
      if (strcmp(token,".") != 0)
	m_userPath << QString(TildeExpand(token));
      token = strtok(NULL,PATH_DELIM);
    }
    rescanPath();
  }

  std::string Interface::getTotalPath() {
    std::string retpath;
    QStringList totalPath(QStringList() << m_basePath << m_userPath);
    for (int i=0;i<totalPath.size()-1;i++) 
      retpath = retpath + totalPath[i].toStdString() + PATH_DELIM;
    if (totalPath.size() > 0) 
      retpath = retpath + totalPath[totalPath.size()-1].toStdString();
    return retpath;
  }
  
  std::string Interface::getPath() {
    std::string retpath;
    QStringList totalPath(m_userPath);
    for (int i=0;i<totalPath.size()-1;i++) 
      retpath = retpath + totalPath[i].toStdString() + PATH_DELIM;
    if (totalPath.size() > 0) 
      retpath = retpath + totalPath[totalPath.size()-1].toStdString();
    return retpath;
  }
  
  void Interface::rescanPath() {
    if (!m_context) return;
    m_context->flushTemporaryGlobalFunctions();
    for (int i=0;i<m_basePath.size();i++)
      scanDirectory(m_basePath[i].toStdString(),false,"");
    for (int i=0;i<m_userPath.size();i++)
      scanDirectory(m_userPath[i].toStdString(),false,"");
    // Scan the current working directory.
    char cwd[1024];
    getcwd(cwd,1024);
    scanDirectory(std::string(cwd),true,"");
    CWDChanged();
  }
  
  /*.......................................................................
   * Search backwards for the potential start of a filename. This
   * looks backwards from the specified index in a given string,
   * stopping at the first unescaped space or the start of the line.
   *
   * Input:
   *  string  const char *  The string to search backwards in.
   *  back_from      int    The index of the first character in string[]
   *                        that follows the pathname.
   * Output:
   *  return        char *  The pointer to the first character of
   *                        the potential pathname, or NULL on error.
   */
  static char *start_of_path(const char *string, int back_from)
  {
    int i, j;
    /*
     * Search backwards from the specified index.
     */
    for(i=back_from-1; i>=0; i--) {
      int c = string[i];
      /*
       * Stop on unescaped spaces.
       */
      if(isspace((int)(unsigned char)c)) {
	/*
	 * The space can't be escaped if we are at the start of the line.
	 */
	if(i==0)
	  break;
	/*
	 * Find the extent of the escape characters which precedes the space.
	 */
	for(j=i-1; j>=0 && string[j]=='\\'; j--)
	  ;
	/*
	 * If there isn't an odd number of escape characters before the space,
	 * then the space isn't escaped.
	 */
	if((i - 1 - j) % 2 == 0)
	  break;
      } 
      else if (!isalpha(c) && !isdigit(c) && (c != '_') && (c != '.') && (c != '\\') && (c != '/'))
	break;
    };
    return (char *)string + i + 1;
  }

  std::vector<std::string> Interface::GetCompletions(std::string line, 
						     int word_end, 
						     std::string &matchString) {
    std::vector<std::string> completions;
    /*
     * Find the start of the filename prefix to be completed, searching
     * backwards for the first unescaped space, or the start of the line.
     */
    char *start = start_of_path(line.c_str(), word_end);
    char *tmp;
    int mtchlen;
    mtchlen = word_end - (start-line.c_str());
    tmp = (char*) malloc(mtchlen+1);
    memcpy(tmp,start,mtchlen);
    tmp[mtchlen] = 0;
    matchString = std::string(tmp);
    
    /*
     *  the preceeding character was not a ' (quote), then
     * do a command expansion, otherwise, do a filename expansion.
     */
    if (start[-1] != '\'') {
      std::vector<std::string> local_completions;
      std::vector<std::string> global_completions;
      int i;
      local_completions = m_context->getCurrentScope()->getCompletions(std::string(start));
      global_completions = m_context->getGlobalScope()->getCompletions(std::string(start));
      for (i=0;i<local_completions.size();i++)
	completions.push_back(local_completions[i]);
      for (i=0;i<global_completions.size();i++)
	completions.push_back(global_completions[i]);
      std::sort(completions.begin(),completions.end());
      return completions;
    } else {
#ifdef WIN32
      HANDLE hSearch;
      WIN32_FIND_DATA FileData;
      std::string pattern(tmp);
      pattern.append("*");
      hSearch = FindFirstFile(pattern.c_str(),&FileData);
      if (hSearch != INVALID_HANDLE_VALUE) {
	// Windows does not return any part of the path in the completion,
	// So we need to find the base part of the pattern.
	int lastslash;
	std::string prefix;
	lastslash = pattern.find_last_of("/");
	if (lastslash == -1) {
	  lastslash = pattern.find_last_of("\\");
	}
	if (lastslash != -1)
	  prefix = pattern.substr(0,lastslash+1);
	completions.push_back(prefix + FileData.cFileName);
	while (FindNextFile(hSearch, &FileData))
	  completions.push_back(prefix + FileData.cFileName);
      }
      FindClose(hSearch);
      return completions;
#else
      glob_t names;
      std::string pattern(tmp);
      pattern.append("*");
      glob(pattern.c_str(), GLOB_MARK, NULL, &names);
      int i;
      for (i=0;i<names.gl_pathc;i++) 
	completions.push_back(names.gl_pathv[i]);
      globfree(&names);
      free(tmp);
      return completions;
#endif
    }
  }

  void Interface::setBasePath(QStringList pth) {
    m_basePath = pth;
  }

  void Interface::setUserPath(QStringList pth) {
    m_userPath = pth;
  }
  
  static QString mexExtension() {
#ifdef Q_OS_LINUX
    return "fmxglx";
#endif
#ifdef Q_OS_MACX
    return "fmxmac";
#endif
#ifdef Q_OS_WIN32
    return "fmxw32";
#endif
    return "fmx";
  }
  
  void Interface::scanDirectory(std::string scdir, bool tempfunc,
				std::string prefix) {
    QDir dir(QString::fromStdString(scdir));
    dir.setFilter(QDir::Files|QDir::Dirs|QDir::NoDotAndDotDot);
    dir.setNameFilters(QStringList() << "*.m" << "*.p" 
		       << "@*" << "private" << "*."+mexExtension());
    QFileInfoList list(dir.entryInfoList());
    for (unsigned i=0;i<list.size();i++) {
      QFileInfo fileInfo(list.at(i));
      std::string fileSuffix(fileInfo.suffix().toStdString());
      std::string fileBaseName(fileInfo.baseName().toStdString());
      std::string fileAbsoluteFilePath(fileInfo.absoluteFilePath().toStdString());
      if (fileSuffix == "m" || fileSuffix == "M") 
	if (prefix.empty())
	  procFileM(fileBaseName,fileAbsoluteFilePath,tempfunc);
	else
	  procFileM(prefix + ":" + fileBaseName,fileAbsoluteFilePath,tempfunc);
      else if (fileSuffix == "p" || fileSuffix == "P")
	if (prefix.empty())
	  procFileP(fileBaseName,fileAbsoluteFilePath,tempfunc);
	else
	  procFileP(prefix + ":" + fileBaseName,fileAbsoluteFilePath,tempfunc);
      else if (fileBaseName[0] == '@')
	scanDirectory(fileAbsoluteFilePath,tempfunc,fileBaseName);
      else if (fileBaseName == "private") 
	scanDirectory(fileAbsoluteFilePath,tempfunc,fileAbsoluteFilePath);
      else
	procFileMex(fileBaseName,fileAbsoluteFilePath,tempfunc);
    }
  }
  
  void Interface::procFileM(std::string fname, std::string fullname, bool tempfunc) {
    MFunctionDef *adef;
    adef = new MFunctionDef();
    adef->name = fname;
    adef->fileName = fullname;
    m_context->insertFunctionGlobally(adef, tempfunc);
  }
  
  void Interface::procFileP(std::string fname, std::string fullname, bool tempfunc) {
    MFunctionDef *adef;
    // Open the file
    try {
      File *f = new File(fullname.c_str(),"rb");
      Serialize *s = new Serialize(f);
      s->handshakeClient();
      s->checkSignature('p',1);
      adef = ThawMFunction(s);
      adef->pcodeFunction = true;
      delete f;
      m_context->insertFunctionGlobally(adef, tempfunc);
    } catch (Exception &e) {
    }
  }

  void Interface::procFileMex(std::string fname, std::string fullname, bool tempfunc) {
    MexFunctionDef *adef;
    adef = new MexFunctionDef(fullname);
    adef->name = fname;
    if (adef->LoadSuccessful())
      m_context->insertFunctionGlobally((MFunctionDef*)adef,tempfunc);
    else
      delete adef;
  }
}