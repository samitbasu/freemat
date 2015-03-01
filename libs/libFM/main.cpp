#include <boost/timer/timer.hpp>
#include <boost/unordered_map.hpp>
#include <boost/graph/graph_traits.hpp>
#include <boost/graph/adjacency_list.hpp>
#include <boost/graph/topological_sort.hpp>
#include <unordered_map>
#include <readline/readline.h>
#include <readline/history.h>
#include "Object.hpp"
#include "DoubleType.hpp"
#include "Scanner.hpp"
#include "Parser.hpp"
#include "ThreadContext.hpp"
#include "AllTypes.hpp"
#include "Compiler.hpp"
#include "Assembler.hpp"
#include "VM.hpp"
#include "fnv.hh"
#include "HashMap.hpp"
#include "Symbol.hpp"
#include "GarbageCollector.hpp"
#include "HandleClass.hpp"
#include "FileSystem.hpp"
#include "Debug.hpp"

//#include <valarray>

// What is the strategy for get?
//
// get(ObjectVector)
//
// Option 1. Convert all arguments to index arrays.
//   Each Type object has a "asIndex" function.
//   Something like:
//      Object asIndex(const Object &a, const Tuple &dims)
//  The index type object can then be used to index the output.
//  Need to make a decision about slicing.
//

using namespace FM;

typedef std::pair<int, int> Edge;

struct cycle_detector : public boost::dfs_visitor<>
{
  cycle_detector(std::list<Edge > &bad_list) : _bad_list(bad_list) {}
  template <class Edge, class Graph>
  void back_edge(Edge e, Graph& g) {
    std::cout << "Found back edge " << e << "\n";
    std::cout << "Source: " << source(e,g) << "\n";
    std::cout << "Target: " << target(e,g) << "\n";
    _bad_list.push_back(std::make_pair(source(e,g),target(e,g)));
  }
protected:
  std::list<Edge > &_bad_list;
};


void compileModule(ThreadContext *ctxt, const FMString &name, HashMap<Object> &classes)
{
  std::cout << "Compiling module: " << name << "\n";
  try {
    bool failed;
    FMString body = ReadFileIntoString(name+".m",failed);
    body += "\n\n";
    ctxt->_compiler->compile(body,resolve_full_path(name));
    Module *mod = ctxt->_compiler->module();
    if (mod)
      {
	Object p = ctxt->_asm->run(mod);
	Disassemble(ctxt,p);
	std::cout << "Compile: \n";
	std::cout << p << "\n";
	if (ctxt->_module->ro(p)->m_modtype == ClassdefModuleType)
	  classes.insert(std::make_pair(ctxt->_string->makeString(name),p));
	  // {
	  //   // Check for dependencies...
	  //   ctxt->_module->deref(p);
	  // }
	else
	  ctxt->_globals->insert(std::make_pair(name,p));
      }
    else
      {
	std::cout << "   Module empty\n";
      }
  } catch (const FM::Exception &e) {
    std::cout << "Exception: " << e.msg() << "\n";
  }
}

Object strcmp(const Object &args, int nargout, ThreadContext *ctxt) {
  if (args.count() != 2) throw Exception("strcmp requires two arguments");
  const Object &x = ctxt->_list->first(args);
  const Object &y = ctxt->_list->second(args);
  if (x.isString() && y.isString())
    return ctxt->_list->makeScalar(ctxt->_bool->makeScalar(ctxt->_string->str(x) == ctxt->_string->str(y)));
  return ctxt->_list->makeScalar(ctxt->_bool->makeScalar(false));
}

Object size(const Object &args, int nargout, ThreadContext *ctxt) {
  Tuple dims(Tuple::canonicalForm(ctxt->_list->ro(args)[0].dims()));
  Object ret(ctxt->_double->makeMatrix(1,dims.dimensions()));
  for (int i=0;i<dims.dimensions();i++)
    ctxt->_double->rw(ret)[i] = dims.dimension(i);
  return ctxt->_list->makeScalar(ret);
}

Object print(const Object &args, int nargout, ThreadContext *ctxt) {
  ctxt->_io->output("\n\nPrint:" + args.description() + "\n\n");
  return ctxt->_list->empty();
}

Object handir(const Object &args, int nargout, ThreadContext *ctxt) {
  for (auto p : ctxt->_handles) {
    ctxt->_io->output(" Handle class: " + Stringify(p) + "\n");
  }
  return ctxt->_list->empty();
}

Object numel(const Object &args, int nargout, ThreadContext *ctxt) {
  return ctxt->_list->makeScalar(ctxt->_double->makeScalar(ctxt->_list->ro(args)[0].count()));
}


Object to_int8(const Object &args, int nargout, ThreadContext *ctxt) {
  return ctxt->_list->makeScalar(ctxt->_int8->convert(ctxt->_list->ro(args)[0]));
}

Object to_int16(const Object &args, int nargout, ThreadContext *ctxt) {
  return ctxt->_list->makeScalar(ctxt->_int16->convert(ctxt->_list->ro(args)[0]));
}

Object to_int32(const Object &args, int nargout, ThreadContext *ctxt) {
  return ctxt->_list->makeScalar(ctxt->_int32->convert(ctxt->_list->ro(args)[0]));
}

Object to_int64(const Object &args, int nargout, ThreadContext *ctxt) {
  return ctxt->_list->makeScalar(ctxt->_int64->convert(ctxt->_list->ro(args)[0]));
}

Object to_uint8(const Object &args, int nargout, ThreadContext *ctxt) {
  return ctxt->_list->makeScalar(ctxt->_uint8->convert(ctxt->_list->ro(args)[0]));
}

Object to_uint16(const Object &args, int nargout, ThreadContext *ctxt) {
  return ctxt->_list->makeScalar(ctxt->_uint16->convert(ctxt->_list->ro(args)[0]));
}

Object to_uint32(const Object &args, int nargout, ThreadContext *ctxt) {
  return ctxt->_list->makeScalar(ctxt->_uint32->convert(ctxt->_list->ro(args)[0]));
}

Object to_uint64(const Object &args, int nargout, ThreadContext *ctxt) {
  return ctxt->_list->makeScalar(ctxt->_uint64->convert(ctxt->_list->ro(args)[0]));
}

Object to_double(const Object &args, int nargout, ThreadContext *ctxt) {
  return ctxt->_list->makeScalar(ctxt->_double->convert(ctxt->_list->ro(args)[0]));
}

Object to_single(const Object &args, int nargout, ThreadContext *ctxt) {
  return ctxt->_list->makeScalar(ctxt->_single->convert(ctxt->_list->ro(args)[0]));
}

Object backtrace(const Object &args, int nargout, ThreadContext *ctxt) {
  return ctxt->_list->makeScalar(ctxt->_vm->backtrace());
}

Object dbup(const Object &args, int nargout, ThreadContext *ctxt) {
  ctxt->_vm->dbshift(-1);
  return ctxt->_list->empty();
}

Object dbdown(const Object &args, int nargout, ThreadContext *ctxt) {
  ctxt->_vm->dbshift(1);
  return ctxt->_list->empty();
}

Object dblist(const Object &args, int nargout, ThreadContext *ctxt) {
  std::lock_guard<std::mutex> guard(*(ctxt->_lock));
  for (auto p : *ctxt->_bps) {
    std::cout << "Breakpoint " << p->id << " Name " << p->frame_name << " Line "
	      << p->line_number << "\n";
  }
  return ctxt->_list->empty();
}

// Create an addlistener method for the handle class

Object classfunc(const Object &args, int nargout, ThreadContext *ctxt) {
  const Object &x = ctxt->_list->ro(args)[0];
  if (!x.isClass())
    return ctxt->_list->makeScalar(ctxt->_string->makeString(x.type()->name()));
  else
    return ctxt->_list->makeScalar(ctxt->_class->className(x));
}

int main(int argc, char *argv[])
{

  assert(sizeof(Complex<char>) == 2*sizeof(char));

  StdIOTermIF io;
  std::mutex *lock = new std::mutex;
  std::map<FMString,Object> *globals = new std::map<FMString,Object>();
  BPSet *bps = new BPSet;
  
  ThreadContext *ctxt = BuildNewThreadContext(&io);
  ctxt->_globals = globals; // shared by all
  ctxt->_lock = lock; // shared by all
  ctxt->_bps = bps; // shared by all

  makeHandleClass(ctxt);
  //  makeListenerClass(ctxt);

  boost::timer::cpu_timer timer;

  boost::filesystem::path cwd(boost::filesystem::current_path());
  boost::filesystem::directory_iterator p(cwd);

  HashMap<Object> classes;

  while (p != boost::filesystem::directory_iterator()) {
    std::cout << "File: " << *p << " Extension " << p->path().extension() << " Stem " << p->path().stem() << "\n";
    if (p->path().extension() == ".m" || p->path().extension() == ".M")
      {
	FMString func = p->path().stem().string();
	std::cout << "  Parsing function " << func << "\n";
	compileModule(ctxt,func,classes);
      }
    ++p;
  }

  std::cout << "********************************************************************************\n";
  std::cout << "*  Classes\n";
  for (auto i=classes.begin();i!=classes.end();++i)
    std::cout << "Class: " << i->first << " " << i->second << "\n";
  std::cout << "********************************************************************************\n";

  // Use Boost to analyze the dependencies
  FMMap<FMString,int> name_to_node_map;
  FMMap<int,FMString> node_to_name_map;
  int nodectr = 0;
  for (auto i=classes.begin();i!=classes.end();++i)
    {
      std::cout << "Assigning class " << i->first << " to node " << nodectr << "\n";
      name_to_node_map[ctxt->_string->str(i->first)] = nodectr;
      node_to_name_map[nodectr] = ctxt->_string->str(i->first);
      nodectr++;
    }

  // Next, build a list of edges
  std::vector<Edge > edge_list;

  for (auto i=classes.begin();i!=classes.end();++i)
    {
      int src = name_to_node_map[ctxt->_string->str(i->first)];
      Object dependencies = ctxt->_module->ro(i->second)->m_dependencies;
      for (int j=0;j<dependencies.count();j++) {
	Object dependency = ctxt->_list->ro(dependencies)[j];
	FMString dependency_name = ctxt->_string->str(dependency);
	if (name_to_node_map.contains(dependency_name))
	  {
	    int dst = name_to_node_map[dependency_name];
	    std::cout << "Class " << src << " depends on class " << dst << "\n";
	    edge_list.push_back(std::make_pair(dst,src));
	  }
     }
    }

  // Check for cycles
  
  
  // Use BOOST to find the visitation order


  {
    using namespace boost;
    typedef adjacency_list<vecS, vecS, bidirectionalS, property<vertex_color_t, default_color_type> > Graph;
    Graph g(edge_list.begin(), edge_list.end(), nodectr);
    typedef graph_traits<Graph>::vertex_descriptor Vertex;
    typedef std::list<Vertex> VisitOrder;
    VisitOrder visit_order;

    std::list<Edge > bad_list;
    cycle_detector vis(bad_list);
    boost::depth_first_search(g, visitor(vis));
    std::cout << "The graph has a cycle? " << (bad_list.size() > 0) << "\n";
    for (auto i=bad_list.begin();i!=bad_list.end();++i)
      {
	std::cout << "Classes " + node_to_name_map[i->first] + " and " + 
	  node_to_name_map[i->second] + " have a cyclic dependency\n";
      }
    if (bad_list.size() == 0) {
      boost::topological_sort(g, std::front_inserter(visit_order));
      for (auto i=visit_order.begin(); i!= visit_order.end(); ++i) {
	std::cout << " Visit " << node_to_name_map[*i] << "\n";
	Object mod = classes.at(ctxt->_string->makeString(node_to_name_map[*i]));
	ctxt->_module->deref(mod);
      }
    }
  }


  // For now - hard code a single function to preload
  /*
  ctxt->_globals->insert(std::make_pair("three",compileFunc(ctxt,"three")));
  ctxt->_globals->insert(std::make_pair("add",compileFunc(ctxt,"add")));
  ctxt->_globals->insert(std::make_pair("fixa",compileFunc(ctxt,"fixa")));
  ctxt->_globals->insert(std::make_pair("dima",compileFunc(ctxt,"dima")));
  ctxt->_globals->insert(std::make_pair("t4",compileFunc(ctxt,"t4")));
  ctxt->_globals->insert(std::make_pair("swit",compileFunc(ctxt,"swit")));
  ctxt->_globals->insert(std::make_pair("exc",compileFunc(ctxt,"exc")));
  ctxt->_globals->insert(std::make_pair("excb",compileFunc(ctxt,"excb")));
  ctxt->_globals->insert(std::make_pair("bind",compileFunc(ctxt,"bind")));
  */


  // TODO - Fix this.
  // This isn't completely correct -- we should delay metaclass construction until after
  // the functions have been parsed.  Otherwise, e.g., pi.m may not be parsed before aclass.m.
  //  compileModule(ctxt,"sclass");

  // compileModule(ctxt,"mab");
  // compileModule(ctxt,"foo");
  // compileModule(ctxt,"bar");
  // compileModule(ctxt,"sclass");
  // compileModule(ctxt,"grok");

  // Object sclassMeta = ctxt->_globals->at("?sclass");
  // Object soo = ctxt->_meta->construct(sclassMeta);
  // ctxt->_globals->insert(std::make_pair("soo",soo));

  //  std::cout << "Symbol flags size: " << sizeof(symbol_flags_t) << "\n";

  //  compileFunc(ctxt,"sclass");
  
  //  compileModule(ctxt,"localfn");
  //  compileModule(ctxt,"three");

  //  compileModule(ctxt,"nest1");
  //  compileModule(ctxt,"counter");
  //  compileModule(ctxt,"makeParabola");

  {
    // Create a variable
    Object a = ctxt->_double->makeScalar(3.14);
    // Create a capture of it
    Object cap1 = ctxt->_captured->makeScalar();
    ctxt->_captured->set(cap1,a);
    // Insert cap1 as a variable in the global workspace
    ctxt->_globals->insert(std::make_pair("cap1",cap1));

    // Insert a copy of cap1 as another variable in the global workspace
    Object cap2 = ctxt->_captured->makeScalar();
    cap2 = cap1;
    ctxt->_globals->insert(std::make_pair("cap2",cap2));
  }

  ctxt->_globals->insert(std::make_pair("size",ctxt->_builtin->makeBuiltin("size",size)));
  ctxt->_globals->insert(std::make_pair("print",ctxt->_builtin->makeBuiltin("print",print)));
  ctxt->_globals->insert(std::make_pair("handir",ctxt->_builtin->makeBuiltin("handir",handir)));
  ctxt->_globals->insert(std::make_pair("strcmp",ctxt->_builtin->makeBuiltin("strcmp",strcmp)));
  ctxt->_globals->insert(std::make_pair("numel",ctxt->_builtin->makeBuiltin("numel",numel)));
  ctxt->_globals->insert(std::make_pair("int8",ctxt->_builtin->makeBuiltin("int8",to_int8)));
  ctxt->_globals->insert(std::make_pair("int16",ctxt->_builtin->makeBuiltin("int16",to_int16)));
  ctxt->_globals->insert(std::make_pair("int32",ctxt->_builtin->makeBuiltin("int32",to_int32)));
  ctxt->_globals->insert(std::make_pair("int64",ctxt->_builtin->makeBuiltin("int64",to_int64)));
  ctxt->_globals->insert(std::make_pair("uint8",ctxt->_builtin->makeBuiltin("uint8",to_uint8)));
  ctxt->_globals->insert(std::make_pair("uint16",ctxt->_builtin->makeBuiltin("uint16",to_uint16)));
  ctxt->_globals->insert(std::make_pair("uint32",ctxt->_builtin->makeBuiltin("uint32",to_uint32)));
  ctxt->_globals->insert(std::make_pair("uint64",ctxt->_builtin->makeBuiltin("uint64",to_uint64)));
  ctxt->_globals->insert(std::make_pair("double",ctxt->_builtin->makeBuiltin("double",to_double)));
  ctxt->_globals->insert(std::make_pair("single",ctxt->_builtin->makeBuiltin("single",to_single)));
  ctxt->_globals->insert(std::make_pair("backtrace",ctxt->_builtin->makeBuiltin("backtrace",backtrace)));
  ctxt->_globals->insert(std::make_pair("dbup",ctxt->_builtin->makeBuiltin("dbup",dbup)));
  ctxt->_globals->insert(std::make_pair("dbdown",ctxt->_builtin->makeBuiltin("dbdown",dbdown)));
  ctxt->_globals->insert(std::make_pair("dblist",ctxt->_builtin->makeBuiltin("dblist",dblist)));
  ctxt->_globals->insert(std::make_pair("class",ctxt->_builtin->makeBuiltin("class",classfunc)));
  ctxt->_globals->insert(std::make_pair("gc",ctxt->_builtin->makeBuiltin("gc",builtin_gc)));

  // Global symbols
  for (auto i=ctxt->_globals->begin(); i != ctxt->_globals->end(); ++i)
    {
      std::cout << "Global symbol: " << i->first << ": " << i->second.brief() << "\n";
    }

  // Look up the global symbol for cap1
  {
    Object cap1ref = ctxt->_globals->at("cap1");
    ctxt->_captured->set(cap1ref,ctxt->_double->makeScalar(2.78));
  }

  while (1)
    {
      char *p = readline("--> ");
      if (p && *p)
	add_history(p);
      if (!p)
	return 0;
      FMString body(p);
      body += "\n\n";
      try {
	ctxt->_compiler->compile(body);
	Module *mod = ctxt->_compiler->module();
	if (mod)
	  {
	    Object p = ctxt->_asm->run(mod);
	    std::cout << "Code object: " << p.description() << "\n";
	    Disassemble(ctxt,p);
	    timer.start();
	    Object params = ctxt->_list->empty();
	    ctxt->_vm->executeModule(p,params);
	    timer.stop();
	    std::cout << " Execution time " << timer.elapsed().wall/1.0e9 << "\n";
	  }
      } catch (const FM::Exception &e) {
	std::cout << "Exception: " << e.msg() << "\n";
      }
    }

  return 0;


}
