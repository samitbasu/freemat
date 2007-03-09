#ifndef __Tree_hpp__
#define __Tree_hpp__

#include <string>
#include <stack>
#include <vector>
#include <iostream>
#include "Token.hpp"

using namespace std;

class tree;

class tree_node {
public:
  Token node;
  int owners;
  vector<tree> children;
  void print();
  void Rename(byte newtok);
  tree_node* getCopy();
  tree_node();
};

class tree {
  tree_node* tptr;
public:
  tree();
  tree(tree_node* ptr) : tptr(ptr) {}
  tree(const tree& copy);
  ~tree();
  tree_node* ptr() {return tptr;}
  void print();
  void Rename(byte newtok) {if (tptr) tptr->Rename(newtok);}
  bool valid() {return (tptr != NULL);}
  void operator=(const tree &copy);
};

tree mkLeaf(const Token& tok);
tree mkLeaf(byte a);
tree mkNode(const Token& tok, tree arg1, tree arg2);
tree mkNode(const Token& tok, tree arg1);
void addChild(tree &root, tree child);
void addChild(tree &root, tree child1, tree child2);

#endif