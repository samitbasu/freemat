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

#ifndef __Context_hpp__
#define __Context_hpp__


#include "Scope.hpp"
#include "Array.hpp"
#include "FunctionDef.hpp"
#include "Exception.hpp"
#include "Types.hpp"
#include <vector>
#include <QList>
#include <QMutex>
#include <stdarg.h>
#include "DebugStream.hpp"

/**
 * This class represents a reference to an array in a scope
 */
class ArrayReference {
  Array *m_ptr;
  bool m_global;
  ScopePtr m_scope;
public:
  inline ArrayReference() : m_ptr(NULL), m_global(false), m_scope(NULL) {
  }
  inline ArrayReference(Array *ptr) :  m_ptr(ptr), m_global(false), m_scope(NULL) {
  }
  inline ArrayReference(Array *ptr, bool global, ScopePtr scope) :
    m_ptr(ptr), m_global(global), m_scope(scope) {
    if (m_global)
      m_scope->lock();
  }
  inline ~ArrayReference() {
    if (m_global)
      m_scope->unlock();
  }
  inline ArrayReference(const ArrayReference& copy) {
    m_ptr = copy.m_ptr;
    m_global = copy.m_global;
    m_scope = copy.m_scope;
    if (m_global)
      m_scope->lock();
  }
  inline ArrayReference& operator=(Array* ptr) {
    if (m_global)
      m_scope->unlock();
    m_ptr = ptr;
    m_global = false;
    m_scope = NULL;
    return *this;
  }
  inline ArrayReference& operator=(const ArrayReference& copy) {
    if (m_global)
      m_scope->unlock();
    m_ptr = copy.m_ptr;
    m_global = copy.m_global;
    m_scope = copy.m_scope;
    if (m_global)
       m_scope->lock();
    return *this;
  }
  inline Array* pointer() {
    return m_ptr;
  }
  inline Array& operator*() {
    return *m_ptr;
  }
  inline Array* operator->() {
    return m_ptr;
  }
  inline bool valid() {
    return (m_ptr != NULL);
  }
};

/**
 * A Context is a stack of scopes with the (peculiar) property that
 * the top and bottom of the stack (global and local scopes respectively)
 * are searched regularly.  The context is responsible for determining
 * if variables and functions exist, and if so, for returning their
 * values and accepting modifications to them.  A context also keeps
 * track of loop depth.  Furthermore, the context (starting with 3.1)
 * now "owns" all of the functions that are defined.  These used to be
 * tracked with the scopes, but that lead to a memory leak of FunctionDef
 * objects.
 */

typedef FM::SymbolTable<FuncPtr> CodeTable;

class Context {
  /**
   * The normal stack of scopes.
   */
  QVector<ScopePtr> scopestack;
  /**
   * The stack of scopes that have been "bypassed"
   */
  QList<ScopePtr> bypassstack;
  /**
   * The table of functions
   */
  CodeTable codeTab;
  /**
   * List of functions that are "temporary" and should be flushed
   */
  StringVector tempFunctions;
  /**
   * The set of builtin functions
   */
  CodeTable builtinTab;
  /**
   * The set of captured mfunctions
   */ 
  QMultiMap<QString,FuncPtr> mFunctions;
  /**
   * Mutex to control access to this context class.
   */
  QMutex mutex;
  /**
   * Pointer to current scope
   */
  ScopePtr bottomScope;
  /**
   * Pointer to global scope
   */
  ScopePtr topScope;
public:
  /**
   * Create a context and initialize it with a global scope and a 
   * base scope.
   */
  Context(ScopePtr global) : mutex(QMutex::Recursive) {
    scopestack.push_back(global);
    pushScope("base");
    topScope = scopestack.front();
    bottomScope = scopestack.back();
  }

  /**
   * Get the pointer to the mutex that protects this context.
   * This mutex only needs to be used when the GUI thread wants
   * to access the context (which is "owned" by the Interpreter
   * thread).
   */
  inline QMutex* getMutex() {
    return &mutex;
  }

  /**
   * Bypass the prescribed number of scopes.  These scopes are
   * placed on the bypassstack.  This effectively makes a different
   * scope active, and then allows us to restore the bypass scopes.
   * a count of -1 means all scopes are bypassed (except the base scope)
   */
  inline void bypassScope(int count) {
    if (count < 0)
      count = scopestack.size();
    while ((count > 0) && (scopestack.back()->getName() != "base")) {
      bypassstack.push_back(scopestack.back());
      scopestack.pop_back();
      count--;
    }
    bottomScope = scopestack.back();
  }
  inline void restoreScope(int count) {
    for (int i=0;i<count;i++) {
      scopestack.push_back(bypassstack.back());
      bypassstack.pop_back();
    }
    bottomScope = scopestack.back();
  }
  /**
   * Every call to bypassScope should be matched by a call to 
   * restoreBypassedScopes, or memory leaks will occur.
   */
  inline void restoreBypassedScopes() {
    for (int i=0;i<bypassstack.size();i++)
      scopestack.push_back(bypassstack[bypassstack.size()-1-i]);
    bypassstack.clear();
    bottomScope = scopestack.back();
  }
  /**
   * Push the given scope onto the bottom of the scope stack.
   */
  inline void pushScope(QString name, bool nestflag = false) {
    if (scopestack.size() > 100)
      throw Exception("Allowable stack depth exceeded...");
    scopestack.push_back(new Scope(name,nestflag));
    bottomScope = scopestack.back();
  }
  /**
   * Pop the bottom scope off of the scope stack.  The scope is
   * deleted.
   * Throws an Exception if the global scope is popped.
   */
  inline void popScope() {
    if (scopestack.size() == 1)
      throw Exception("Attempt to pop global scope off of context stack!");
    scopestack.pop_back();
    delete bottomScope;
    bottomScope = scopestack.back();
  }
  inline int scopeDepth() {
    return scopestack.size();
  }
  /**
   * Insert the given variable into the right scope - the global
   * scope if the array is in the global list, and mangled in
   * global list if it is persistent.  
   */
  inline void insertVariable(const QString& varName, const Array& var) {
    ScopePtr active;
    QString mapName;

    if (bottomScope->isVariablePersistent(varName)) {
      mapName = bottomScope->getMangledName(varName);
      active = topScope;
    } else if (bottomScope->isVariableGlobal(varName)) {
      mapName = varName;
      active = topScope;
    } else {
      bottomScope->insertVariable(varName,var);
      return;
    }
    active->insertVariable(mapName,var);
  }

  /**
   * Insert a variable into the local scope - do not check the
   * global list.
   */
  inline void insertVariableLocally(QString varName, const Array& var) {
    bottomScope->insertVariable(varName,var);
  }
  /**
   * Return a pointer to the given variable.  The search
   * logic is:
   *   - If the variable is persistent in the current scope
   *     (at the bottom of the scope stack), mangle its name
   *     using the scope, and look for it in the global scope.
   *   - If the variable is global in the current scope (at the
   *     bottom of the scope stack, look for it in the global
   *     scope.
   *   - Look for the variable in the local scope.
   * If the variable is not found, an empty variable is constructed
   * with the given name, and inserted into the scope that was
   * searched.  A pointer to this newly created variable is returned.
   */
  inline ArrayReference lookupVariable(const QString& varName) {
    ScopePtr active;
    QString mapName;
    bool global = false;
    if (bottomScope->isVariablePersistent(varName)) {
      mapName = bottomScope->getMangledName(varName);
      active = topScope;
    } else if (bottomScope->isVariableGlobal(varName)) {
      mapName = varName;
      active = topScope;
      global = true;
    } else {
      Array *dp = bottomScope->lookupVariable(varName);
      if (!dp && bottomScope->isnested()) {
	// If the variable is not defined in the current scope,
	// loop up through the scope stack, checking for nested
	// scopes that may have the variable defined.
	int i=scopestack.size()-2;
	while ((!dp) && (i>=0) && scopestack[i]->nests(scopestack[i+1]->getName())) {
	  dp = scopestack[i]->lookupVariable(varName);
	  if (!dp) i--;
	}
	if (dp) 
	  return (ArrayReference(dp,false,scopestack[i]));
	else 
	  return (ArrayReference(dp,false,bottomScope));
      } else 
	return (ArrayReference(dp,false,bottomScope));
    }
    return (ArrayReference(active->lookupVariable(mapName),global,active));
  }
  inline bool variableLocalToCurrentScope(QString varName) {
    return bottomScope->variableLocal(varName);
  }
  inline void setVariablesAccessed(StringVector va) {
    bottomScope->setVariablesAccessed(va);
  }
  inline void setLocalVariablesList(StringVector rv) {
    bottomScope->setLocalVariables(rv);
  }
  /**
   * Look for a variable, but only locally.
   */
  inline Array* lookupVariableLocally(QString varName) {
    return bottomScope->lookupVariable(varName);
  }
  /**
   * Insert a function definition into the code table.
   */
  inline void insertFunction(FuncPtr f, bool temporary) {
    codeTab.insertSymbol(f->name,f);
    if (temporary)
      tempFunctions.push_back(f->name);
  }
  /**
   * Remove a function definition from the code table.
   */
  inline void deleteFunction(const QString& funcName) {
    codeTab.deleteSymbol(funcName);
  }
  /**
   * Flush temporary function definitions from the global context
   */
  inline void flushTemporaryGlobalFunctions() {
    for (int i=0;i<tempFunctions.size();i++)
      deleteFunction(tempFunctions[i]);
    tempFunctions.clear();
  }
  /**
   * Capture a m function
   */
  inline void captureMFunction(FuncPtr ptr) {
    mFunctions.insert(ptr->name,ptr);
  }
  /**
   * Lookup a captured m function
   */
  inline FuncPtr lookupCapturedMFunction(const QString& name, const QString& filename) {
    QList<FuncPtr> matches = mFunctions.values(name);
    for (int i=0;i<matches.size();i++)
      if (((MFunctionDef*)matches[i])->fileName == filename)
	return matches[i];
    return FuncPtr();
  }
  /**
   * Lookup a builtin function
   */
  inline FuncPtr lookupBuiltinFunction(const QString& funcName) {
    FuncPtr* ret = builtinTab.findSymbol(funcName);
    if (ret) 
      return *ret;
    else
      throw Exception("Unable to find builtin function definition " + funcName);
  }
  /**
   * Add a built in function to the global scope with the given name.
   */
  inline void addFunction(QString name, BuiltInFuncPtr fptr, int argc_in, int argc_out, ...) {
    StringVector args;
    va_list argp;
    if (argc_in>0) {
      va_start(argp,argc_out);
      for (int i=0;i<argc_in;i++) {
	const char *t = va_arg(argp, const char *);
	if (!t) {
	  dbout << "addFunction for function " << 
	    name << " is wrong!\n";
	  exit(1);
	}
	args.push_back(t);
      }
      if (va_arg(argp,const char *) != NULL) {
	dbout << "addFunction for function " << 
	  name << " is wrong!\n";
	exit(1);
      }
      va_end(argp);
    }
    BuiltInFunctionDef *f2def;
    f2def = new BuiltInFunctionDef;
    f2def->retCount = argc_out;
    f2def->argCount = argc_in;
    f2def->name = name;
    f2def->fptr = fptr;
    f2def->arguments = args;
    insertFunction(f2def,false);  
    builtinTab.insertSymbol(name,f2def);
  }
  /**
   * Add a special function to the global scope with the given name.
   */
  inline void addSpecialFunction(QString name, SpecialFuncPtr fptr, int argc_in, int argc_out, ...) {
    StringVector args;
    va_list argp;
    if (argc_in>0) {
      va_start(argp,argc_out);
      for (int i=0;i<argc_in;i++) {
	const char *t = va_arg(argp, const char *);
	if (!t) {
	  dbout << "addSpecialFunction for function " << 
	    (name) << " is wrong!\n";
	  exit(1);
	}
	args.push_back(t);
      }
      if (va_arg(argp,const char *) != NULL) {
	dbout << "addSpecialFunction for function " << 
	  (name) << " is wrong!\n";
	exit(1);
      }
      va_end(argp);
    }
    SpecialFunctionDef *f2def;
    f2def = new SpecialFunctionDef;
    f2def->retCount = argc_out;
    f2def->argCount = argc_in;
    f2def->name = name;
    f2def->fptr = fptr;
    f2def->arguments = args;
    insertFunction(f2def,false);
    builtinTab.insertSymbol(name,f2def);
  }
  /**
   * Add a built in function to the global scope with the given name
   * and tag it as a graphics function
   */
  inline void addGfxFunction(QString name, BuiltInFuncPtr fptr, int argc_in, int argc_out, ...) {
    StringVector args;
    va_list argp;
    if (argc_in>0) {
      va_start(argp,argc_out);
      for (int i=0;i<argc_in;i++) {
	const char *t = va_arg(argp, const char *);
	if (!t) {
	  dbout << "addGfxFunction for function " << 
	    (name) << " is wrong!\n";
	  exit(1);
	}
	args.push_back(t);
      }
      if (va_arg(argp,const char *) != NULL) {
	dbout << "addGfxFunction for function " << 
	  (name) << " is wrong!\n";
	exit(1);
      }
      va_end(argp);
    }
    BuiltInFunctionDef *f2def;
    f2def = new BuiltInFunctionDef;
    f2def->retCount = argc_out;
    f2def->argCount = argc_in;
    f2def->name = name;
    f2def->fptr = fptr;
    f2def->arguments = args;
    f2def->graphicsFunction = true;
    insertFunction(f2def,false);  
    builtinTab.insertSymbol(name,f2def);
  }
  /**
   * Add a special function to the global scope with the given name, and
   * tag it as a graphics function
   */
  inline void addGfxSpecialFunction(QString name, SpecialFuncPtr fptr, int argc_in, int argc_out, ...) {
    StringVector args;
    va_list argp;
    if (argc_in>0) {
      va_start(argp,argc_out);
      for (int i=0;i<argc_in;i++) {
	const char *t = va_arg(argp, const char *);
	if (!t) {
	  dbout << "addGfxSpecialFunction for function " << 
	    (name) << " is wrong!\n";
	  exit(1);
	}
	args.push_back(t);
      }
      if (va_arg(argp,const char *) != NULL) {
	dbout << "addGfxSpecialFunction for function " << 
	  (name) << " is wrong!\n";
	exit(1);
      }
      va_end(argp);
    }
    SpecialFunctionDef *f2def;
    f2def = new SpecialFunctionDef;
    f2def->retCount = argc_out;
    f2def->argCount = argc_in;
    f2def->name = name;
    f2def->fptr = fptr;
    f2def->arguments = args;
    f2def->graphicsFunction = true;
    insertFunction(f2def,false);
    builtinTab.insertSymbol(name,f2def);
  }
  
  inline StringVector listAllFunctions() {
    return codeTab.getCompletions("");
  }

  inline StringVector listGlobalVariables() {
    return topScope->listAllVariables();
  }

  inline StringVector listAllVariables() {
    return bottomScope->listAllVariables();
  }

  inline void clearGlobalVariableList() {
    topScope->clearGlobalVariableList();
    bottomScope->clearGlobalVariableList();
  }

  inline void clearPersistentVariableList() {
    topScope->clearPersistentVariableList();
    bottomScope->clearPersistentVariableList();
  }

  inline StringVector getCompletions(const QString& prefix) {
    StringVector local_completions = bottomScope->getCompletions(prefix);
    StringVector global_completions = topScope->getCompletions(prefix);
    StringVector code_completions = codeTab.getCompletions(prefix);
    StringVector completions(local_completions);
    for (int i=0;i<global_completions.size();i++)
      completions.push_back(global_completions[i]);
    for (int i=0;i<code_completions.size();i++)
      completions.push_back(code_completions[i]);
    return completions;
  }

  inline bool lookupFunction(QString funcName, FuncPtr& val) {
    qDebug() << "Looking up " << funcName;
    FuncPtr* ret = codeTab.findSymbol(funcName);
    if (ret) {
      val = *ret;
      return true;
    }
    return false;
  }
    
  /**
   * Add a persistent variable to the local stack.  This involves
   * two steps:
   *   - the name of the variable is added to the persistent variable list
   *     in the current scope.
   *   - the global scope is checked for the mangled name of the 
   *     persistent variable.  If the variable does not exist in the
   *     global scope, then an empty variable is inserted.
   */
  inline void addPersistentVariable(QString var) {
    // Delete local variables with this name
    bottomScope->deleteVariable(var);
    // Delete global variables with this name
    //  topScope->deleteVariable(var);
    bottomScope->addPersistentVariablePointer(var);
  }
  /**
   * Add a variable name into the global variable list of the current
   * scope.  If the variable does not exist in the global scope, an
   * empty variable is added.
   */
  inline void addGlobalVariable(QString var) {
    // Delete local variables with this name
    bottomScope->deleteVariable(var);
    // Delete global persistent variables with this name
    topScope->deleteVariable(bottomScope->getMangledName(var));
    // Add a point in the local scope to the global variable
    bottomScope->addGlobalVariablePointer(var);
  }
  inline void deleteGlobalVariable(QString var) {
    topScope->deleteVariable(var);
  }
  /**
   * Delete a variable if its defined.  Handles global and persistent
   * variables also.
   */
  inline void deleteVariable(QString var) {
    if (isVariableGlobal(var)) {
      topScope->deleteVariable(var);
      bottomScope->deleteGlobalVariablePointer(var);
      return;
    }
    if (isVariablePersistent(var)) {
      topScope->deleteVariable(bottomScope->getMangledName(var));
      bottomScope->deletePersistentVariablePointer(var);
      return;
    }
    bottomScope->deleteVariable(var);
  }
  /**
   * Increment the loop depth counter in the local scope.
   */
  inline void enterLoop() {
    bottomScope->enterLoop();
  }
  /**
   * Decrement the loop depth counter in the local scope.
   */
  inline void exitLoop() {
    bottomScope->exitLoop();
  }
  inline bool isCurrentScopeNested() {
    return bottomScope->isnested();
  }
  inline QString scopeName() {
    return bottomScope->getName();
  }
  inline bool currentScopeNests(QString name) {
    return bottomScope->nests(name);
  }
  inline bool currentScopeVariableAccessed(QString name) {
    return bottomScope->variableAccessed(name);
  }
  /**
   * Returns true if the current (local) scope indicates a
   * non-zero loop depth.
   */
  inline bool inLoop() {
    return bottomScope->inLoop();
  }
  /**
   * Returns true if the given variable is global.
   */
  inline bool isVariableGlobal(const QString& varName) {
    return bottomScope->isVariableGlobal(varName);
  }
  /**
   * Returns true if the given variable is persistent
   */
  inline bool isVariablePersistent(const QString& varName) {
    return bottomScope->isVariablePersistent(varName);
  }
};

#endif
