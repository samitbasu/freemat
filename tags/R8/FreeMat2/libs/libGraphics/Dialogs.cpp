// Copyright (c) 2002, 2003 Samit Basu
// 
// Permission is hereby granted, free of charge, to any person obtaining a 
// copy of this software and associated documentation files (the "Software"), 
// to deal in the Software without restriction, including without limitation 
// the rights to use, copy, modify, merge, publish, distribute, sublicense, 
// and/or sell copies of the Software, and to permit persons to whom the 
// Software is furnished to do so, subject to the following conditions:
// 
// The above copyright notice and this permission notice shall be included 
// in all copies or substantial portions of the Software.
// 
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS 
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, 
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL 
// THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER 
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING 
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER 
// DEALINGS IN THE SOFTWARE.

#include "GraphicsCore.hpp"
#include "Command.hpp"
#include "CLIThread.hpp"

namespace FreeMat {
  ArrayVector HelpwinFunction(int nargout, const ArrayVector& arg) {
    SendGUICommand(new Command(CMD_HelpShow));
    return ArrayVector();
  }

  ArrayVector PickFileFunction(int nargout, const ArrayVector& arg) {
    char *selstring;
    if (arg.size() == 0) 
      selstring = "*";
    else {
      Array tmp(arg[0]);
      selstring = tmp.getContentsAsCString();
    }
    SendGUICommand(new Command(CMD_FilePick));
    Command *reply = GetGUIResponse();
    ArrayVector retval;
    retval.push_back(reply->data);
    delete reply;
    return retval;
  }
}