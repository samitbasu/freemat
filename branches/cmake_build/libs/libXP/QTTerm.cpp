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
#include "QTTerm.hpp"
#include "KeyManager.hpp"
#include <qapplication.h>
#include <qclipboard.h>
#include <math.h>
#include <QtGui>
#include <QEvent>

QTTerm::QTTerm() {
  setMinimumSize(50,50);
  setVerticalScrollBarPolicy(Qt::ScrollBarAlwaysOn);
  setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
  buffer << tagLine();
  cursor_x = 0;
  cursor_y = 0;
  blinkEnable = true;
  m_blink_skip = false;
  m_timer_blink = new QTimer;
  QObject::connect(m_timer_blink, SIGNAL(timeout()), this, SLOT(blink()));
  QSettings settings("FreeMat", "FreeMat");
  scrollback = settings.value("console/scrollback",5000).toInt();
  m_timer_blink->start(settings.value("console/blinkspeed",1000).toInt());
  fnt = QFont("Courier",10);
}

void QTTerm::blink() {
  if (!blinkEnable) return;
  if (m_blink_skip) {
    m_blink_skip = false;
    return;
  }
  buffer[cursor_y].data[cursor_x].toggleCursor();
  viewport()->update();
}

void QTTerm::ensureCursorVisible() {
  // For the cursor to be visible
  // the scroll bar must be at 
  // cursor_y - m_term_height + 1
  int cscroll = verticalScrollBar()->value();
  if ((cscroll < cursor_y) && 
      (cursor_y < (cscroll+m_term_height-1))) return;
  int minval = cursor_y-m_term_height+1;
  verticalScrollBar()->setValue(minval);
}

void QTTerm::focusOutEvent(QFocusEvent *e) {
  QWidget::focusOutEvent(e);
  blinkEnable = false;
  buffer[cursor_y].data[cursor_x].clearCursor();  
  viewport()->update();
}

void QTTerm::focusInEvent(QFocusEvent *e) {
  QWidget::focusInEvent(e);
  buffer[cursor_y].data[cursor_x].setCursor();
  viewport()->update();
  blinkEnable = true;
}

void QTTerm::setChar(char t, bool flush) {
  if (t == '\r') {
    MoveBOL();
    return;
  }
  if (t == '\n') {
    nextLine();
    return;
  }
  blinkEnable = false;
  buffer[cursor_y].data[cursor_x].clearCursor();
  buffer[cursor_y].data[cursor_x++].v = t;
  buffer[cursor_y].data[cursor_x].setCursor();
  if (cursor_x >= m_term_width) {
    nextLine(); 
  } else {
    if (flush) {
      ensureCursorVisible();
      viewport()->update(QRect(((cursor_x)-1)*m_char_w,
			       (cursor_y-verticalScrollBar()->value())*m_char_h,
			       m_char_w*2,m_char_h));
    }
  }
  blinkEnable = true;
  m_blink_skip = true;
}

void QTTerm::setFont(QFont font) {
  QFontInfo fi(font);
  if (!fi.fixedPitch()) 
    QMessageBox::warning(this,"FreeMat",
			 "You have selected a font that is not a fixed pitch.\nThe terminal widget really requires a fixed pitch font to work.");
  fnt = font;
  calcGeometry();
}

QFont QTTerm::getFont() {
  return fnt;
}

void QTTerm::clearCursor() {
  blinkEnable = false;
  buffer[cursor_y].data[cursor_x].clearCursor();
}

void QTTerm::setCursor() {
  buffer[cursor_y].data[cursor_x].setCursor();
  ensureCursorVisible();
  viewport()->update();
  blinkEnable = true;
  m_blink_skip = true;    
}

void QTTerm::MoveDown() {
  clearCursor();
  cursor_y++;
  if (cursor_y >= buffer.size())
    buffer << tagLine();
  if (buffer.size() > scrollback) {
    buffer.pop_front();
    cursor_y--;
  } else {
    int cval = verticalScrollBar()->value();
    verticalScrollBar()->setRange(0,qMax(0,qMax(verticalScrollBar()->maximum(),buffer.size()-m_term_height)));
    verticalScrollBar()->setValue(cval);
  }
  setCursor();
}

void QTTerm::MoveBOL() {
  clearCursor();
  cursor_x = 0;
  setCursor();
}

void QTTerm::MoveUp() {
  clearCursor();
  cursor_y = qMax(0,cursor_y-1);
  setCursor();  
}

void QTTerm::MoveLeft() {
  clearCursor();
  cursor_x = qMax(0,cursor_x-1);
  setCursor();
}

void QTTerm::MoveRight() {
  clearCursor();
  cursor_x = qMin(m_term_width-1,cursor_x+1);
  setCursor();
}

void QTTerm::ClearEOL() {
  for (int j=cursor_x;j<m_term_width-1;j++) {
    buffer[cursor_y].data[j].v = ' ';
    buffer[cursor_y].data[j].flags = 0;
  }
  viewport()->update();
}

void QTTerm::ClearEOD() {
  ClearEOL();
  for (int i=cursor_y+1;i<buffer.size();i++) {
    for (int j=0;j<m_term_width-1;j++) {
      buffer[i].data[j].v = ' ';
      buffer[i].data[j].flags = 0;
    }
  }
  viewport()->update();
}

void QTTerm::ClearDisplay() {
  blinkEnable = false;
  buffer.clear();
  buffer << tagLine();
  cursor_y = 0;
  cursor_x = 0;
  verticalScrollBar()->setRange(0,buffer.size()-m_term_height);
  verticalScrollBar()->setValue(0);
  viewport()->update();
  setCursor();
}

void QTTerm::nextLine() {
  MoveBOL();
  MoveDown();
  cursor_x = 0;
  viewport()->update();
}

void QTTerm::drawLine(int linenum, QPainter *e, int yval) {
  QString outd;
  tagLine todraw(buffer[linenum]);
  char gflags = 0;
  int frag_start = 0;
  for (int col=0;col<m_term_width;col++) {
    tagChar g(todraw.data[col]);
    if (g.mflags() != gflags) {
      drawFragment(e,outd,gflags,yval,frag_start);
      gflags = g.mflags();
      frag_start = col;
      outd.clear();
      outd.append(g.v);
    } else
      outd.append(g.v);
  }
  drawFragment(e,outd,gflags,yval,frag_start);
}

void QTTerm::mousePressEvent( QMouseEvent *e ) {
  // Get the x and y coordinates of the mouse click - map that
  // to a row and column
  int clickcol = e->x()/m_char_w;
  int clickrow = e->y()/m_char_h;
  selectionStart = verticalScrollBar()->value()*m_term_width + clickcol + clickrow*m_term_width;
  selectionStart = qMax(0,selectionStart);
  selectionStop = selectionStart;
}

void QTTerm::clearSelection() {
  // clear the selection bits
  for (int i=0;i<buffer.size();i++) {
    for (int j=0;j<maxlen;j++) {
      buffer[i].data[j].clearSelection();
    }
  }
}

void QTTerm::mouseMoveEvent( QMouseEvent *e ) {
  if (!e->buttons())
    return;
  int x = e->x();
  int y = e->y();
  if (y < 0) 
     verticalScrollBar()->setValue(verticalScrollBar()->value()-1);
   if (y > height())
     verticalScrollBar()->setValue(verticalScrollBar()->value()+1);
  // Get the position of the click
  // to a row and column
  int clickcol = x/m_char_w;
  int clickrow = y/m_char_h;
  selectionStop = verticalScrollBar()->value()*m_term_width + clickcol + clickrow*m_term_width;
  selectionStop = qMax(0,selectionStop);

  clearSelection();

  int lSelectionStart = selectionStart;
  int lSelectionStop = selectionStop;
  if (lSelectionStart > lSelectionStop) 
    qSwap(lSelectionStop,lSelectionStart);

  int sel_row_start = lSelectionStart/m_term_width;
  int sel_col_start = lSelectionStart % m_term_width;
  int sel_row_stop = lSelectionStop/m_term_width;
  int sel_col_stop = lSelectionStop % m_term_width;

  sel_row_start = qMin(sel_row_start,buffer.size()-1);
  sel_row_stop = qMin(sel_row_stop,buffer.size()-1);

  if (sel_row_stop == sel_row_start) {
    for (int j=sel_col_start;j<sel_col_stop;j++)
      buffer[sel_row_start].data[j].setSelection();
  } else {
    for (int j=sel_col_start;j<m_term_width;j++) {
      buffer[sel_row_start].data[j].setSelection();
    }
    for (int i=sel_row_start+1;i<sel_row_stop;i++) 
      for (int j=0;j<m_term_width;j++) {
	buffer[i].data[j].setSelection();
      }
    for (int j=0;j<sel_col_stop;j++)
      buffer[sel_row_stop].data[j].setSelection();
  }
  viewport()->update();
}

void QTTerm::mouseReleaseEvent( QMouseEvent *e ) {
  QClipboard *cb = QApplication::clipboard();
  if (!cb->supportsSelection())
    return;
  cb->setText(getSelectionText(), QClipboard::Selection);
}

void QTTerm::drawFragment(QPainter *paint, QString todraw, char flags, int row, int col) {
  if (todraw.size() == 0) return;
  QRect txtrect(col*m_char_w,row*m_char_h,(col+todraw.size())*m_char_w,m_char_h);
  QPalette qp(qApp->palette());
  if (flags == 0) {
    paint->setPen(qp.color(QPalette::WindowText));
    paint->setBackground(qp.brush(QPalette::Base));
    paint->eraseRect(txtrect);
    paint->drawText(txtrect,Qt::AlignLeft|Qt::AlignTop,todraw);
  } else if (flags & CURSORBIT) {
    paint->setPen(qp.color(QPalette::Base));
    paint->setBackground(Qt::black);
    paint->eraseRect(txtrect);
    paint->drawText(txtrect,Qt::AlignLeft|Qt::AlignTop,todraw);
  } else {
    paint->setPen(qp.color(QPalette::HighlightedText));
    paint->setBackground(qp.brush(QPalette::Highlight));
    paint->eraseRect(txtrect);
    paint->drawText(txtrect,Qt::AlignLeft|Qt::AlignTop,todraw);
  }
}

#ifndef __APPLE__
#define CTRLKEY(x)  else if ((keycode == x) && (e->modifiers() & Qt::ControlModifier))
#else
#define CTRLKEY(x)  else if ((keycode == x) && (e->modifiers() & Qt::MetaModifier))
#endif


bool QTTerm::event(QEvent *e) {
  if (e->type() == QEvent::KeyPress) {
    QKeyEvent *ke = static_cast<QKeyEvent*>(e);
    if (ke->key() == Qt::Key_Tab) {
      emit OnChar(KM_TAB);
      return true;
    }
  }
  return QAbstractScrollArea::event(e);
}

void QTTerm::keyPressEvent(QKeyEvent *e) {
  int keycode = e->key(); 
  if (!keycode) return;
  if (keycode == Qt::Key_Left)
    emit OnChar(KM_LEFT);
  CTRLKEY('Z')
    emit OnChar(KM_CTRLK);
  CTRLKEY('A')
    emit OnChar(KM_CTRLA);
  CTRLKEY('D')
    emit OnChar(KM_CTRLD); 
  CTRLKEY('E')
    emit OnChar(KM_CTRLE);
  CTRLKEY('K')
    emit OnChar(KM_CTRLK);
  CTRLKEY('Y')
    emit OnChar(KM_CTRLY);
  else if (keycode == Qt::Key_Right)
    emit OnChar(KM_RIGHT);
  else if (keycode == Qt::Key_Up)
    emit OnChar(KM_UP);
  else if (keycode == Qt::Key_Down)
    emit OnChar(KM_DOWN);
  else if (keycode == Qt::Key_Delete)
    emit OnChar(KM_DELETE);
  else if (keycode == Qt::Key_Insert)
    emit OnChar(KM_INSERT);
  else if (keycode == Qt::Key_Home)
    emit OnChar(KM_HOME);
  else if (keycode == Qt::Key_End)
    emit OnChar(KM_END);
  else if (keycode == Qt::Key_Return)
    emit OnChar(KM_NEWLINE);
  else if (keycode == Qt::Key_Backspace)
    emit OnChar(KM_BACKSPACE);
  else {
    QByteArray p(e->text().toAscii());
    char key;
    if (!e->text().isEmpty())
      key = p[0];
    else
      key = 0;
    if (key) {
      emit OnChar(key);
      e->accept();
    } else
      e->ignore();
  }
}

void QTTerm::OutputRawString(string txt) {
  for (int i=0;i<txt.size();i++)
    setChar(txt[i],true);
}

void QTTerm::calcGeometry() {
  QFontMetrics fmi(fnt);
  m_char_w = fmi.width("w");
  m_char_h = fmi.height();
  m_term_width = viewport()->width()/m_char_w - 1;
  m_term_height = viewport()->height()/m_char_h;
  emit SetTextWidth(m_term_width);
}

void QTTerm::resizeEvent(QResizeEvent *e) {
  QAbstractScrollArea::resizeEvent(e);
  calcGeometry();
  clearSelection();
  ensureCursorVisible();
  // If we are in a full buffer situation, put the scroller in the right spot
  if (buffer.size() >= scrollback) {
    verticalScrollBar()->setRange(0,buffer.size()-m_term_height);
    verticalScrollBar()->setValue(cursor_y-m_term_height+1);
  } else {
    int cval = verticalScrollBar()->value();
    verticalScrollBar()->setRange(0,qMax(cval,buffer.size()-m_term_height));
    verticalScrollBar()->setValue(cval);
  }
}

void QTTerm::paintEvent(QPaintEvent *e) {
  QPainter p(viewport());
  p.setFont(fnt);
  //  qDebug() << "Current font: " << p.font().toString();
//   p.setBackground(qApp->palette().brush(QPalette::Highlight));
//   p.setPen(qApp->palette().color(QPalette::HighlightedText));
  int offset = verticalScrollBar()->value();
  //  qDebug() << "offset = " << offset;
  for (int i=0;i<m_term_height;i++) 
    if ((i+offset) < buffer.size())
      drawLine(i+offset,&p,i);
}

QString QTTerm::getAllText() {
  QString ret;
  for (int i=0;i<buffer.size();i++) {
    for (int j=0;j<maxlen;j++)
      ret += buffer[i].data[j].v;
    ret += '\n';
  }
  ret.replace(QRegExp(" +\\n"),"\n");
  return ret;
}

QString QTTerm::getSelectionText() {
  QString ret;
  for (int i=0;i<buffer.size();i++) {
    bool lineHasSelectedText = false;
    for (int j=0;j<maxlen;j++)
      if (buffer[i].data[j].selected()) {
	ret += buffer[i].data[j].v;
	lineHasSelectedText = true;
      }
    if (lineHasSelectedText)
      ret += '\n';
  }
  ret.replace(QRegExp(" +\\n"),"\n");
  return ret;
}
